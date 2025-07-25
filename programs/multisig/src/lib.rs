use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    system_instruction,
    program::invoke_signed,
    sysvar::recent_blockhashes::RecentBlockhashes,
};

declare_id!("9ci6bSKQcGTEFGiDTRHacAf84jKuzwE3X5vHBWTDu5nb");

// Move constants outside the module to global scope
const MAX_OWNERS: usize = 10;
const MAX_STORED_NONCES: usize = 100;
const MAX_INSTRUCTION_ACCOUNTS: usize = 10;
const MAX_INSTRUCTION_DATA_SIZE: usize = 1024;

#[program]
pub mod multisig {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, multisig_id: u64, owners: Vec<Pubkey>, threshold: u8) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let creator = &ctx.accounts.creator;

        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.creator = creator.key();
        multisig.multisig_id = multisig_id;
        multisig.used_nonces = Vec::new();

        if threshold > multisig.owners.len() as u8 {
            return Err(ErrorCode::InvalidThreshold.into());
        }
        
        if multisig.owners.is_empty() {
            return Err(ErrorCode::NoOwners.into());
        }

        // Preventing duplicate owners
        let mut unique = std::collections::HashSet::new();
        for owner in &multisig.owners {
            if !unique.insert(owner) {
                return Err(ErrorCode::DuplicateOwners.into());
            }
        }

        Ok(())
    }

    pub fn create_transaction(
      ctx: Context<CreateTransaction>,
      _multisig_id: u64,
      nonce: u64,
      program_id: Pubkey,
      accounts: Vec<TransactionAccount>,
      data: Vec<u8>
    ) -> Result<()> {
        
        let proposer = &ctx.accounts.proposer;

        // Read-only checks first (before mutable borrow)
        require!(
            ctx.accounts.multisig.owners.contains(&proposer.key()),
            ErrorCode::NotAnOwner
        );

        require!(
            !ctx.accounts.multisig.used_nonces.contains(&nonce),
            ErrorCode::NonceAlreadyUsed
        );

        // Validate instruction limits
       require!(
        accounts.len() <= MAX_INSTRUCTION_ACCOUNTS,
        ErrorCode::TooManyAccounts
       );

       require!(
        data.len() <= MAX_INSTRUCTION_DATA_SIZE,
        ErrorCode::InstructionDataTooLarge
       );

        // Optional: Handle system nonce if needed
        if let Some(nonce_account) = &ctx.accounts.nonce_account {
            // Validate nonce authority if needed
            let nonce_account_data = nonce_account.try_borrow_data()
                .map_err(|_| ErrorCode::InvalidNonceAuthority)?;
            
            // Simple validation without full deserialization
            // The nonce account authority is at offset 40 (after version, state, and reserved)
            if nonce_account_data.len() >= 72 {
                let authority_bytes = &nonce_account_data[40..72];
                let authority = Pubkey::try_from(authority_bytes)
                    .map_err(|_| ErrorCode::InvalidNonceAuthority)?;
                
                require_keys_eq!(
                    authority,
                    ctx.accounts.multisig.key(),
                    ErrorCode::InvalidNonceAuthority
                );
            }

            let ix = system_instruction::advance_nonce_account(
                &nonce_account.key(),
                &ctx.accounts.multisig.key(),
            );
            
            // Fix: Create proper seeds array
            let multisig_seeds: &[&[u8]] = &[
                b"multisig",
                &ctx.accounts.multisig.multisig_id.to_le_bytes(),
                &[ctx.bumps.multisig]
            ];
            
            invoke_signed(
                &ix,
                &[
                    nonce_account.to_account_info(),
                    ctx.accounts.multisig.to_account_info(),
                    ctx.accounts.recent_blockhashes.as_ref().unwrap().to_account_info(),
                ],
                &[multisig_seeds],
            )?;
        }

        // Now get mutable references after all immutable operations are done
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        transaction.multisig = multisig.key();
        transaction.proposer = proposer.key();
        transaction.approvals = Vec::new();
        transaction.did_execute = false;
        transaction.nonce = nonce;
        
        transaction.program_id = program_id;
        transaction.accounts = accounts;
        transaction.data = data;

        // Store used nonce with size limit
        if multisig.used_nonces.len() >= MAX_STORED_NONCES {
            multisig.used_nonces.remove(0);
        }
        multisig.used_nonces.push(nonce);

     // Emit event
     emit!(TransactionCreated {
      multisig: multisig.key(),
      transaction: transaction.key(),
      proposer: proposer.key(),
      nonce,
     });
        
        Ok(())
    }

    pub fn approve_transaction(ctx: Context<ApproveTransaction>, _multisig_id: u64, _nonce: u64) -> Result<()> {
        let owner = ctx.accounts.owner.key();
        let multisig = &ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        // Check if signer is an owner
        if !multisig.owners.contains(&owner) {
            return Err(ErrorCode::NotOwner.into());
        }

        // Check if already approved
        if transaction.approvals.contains(&owner) {
            return Err(ErrorCode::AlreadyApproved.into());
        }

        // Check if transaction is already executed
        require!(!transaction.did_execute, ErrorCode::AlreadyExecuted);

        // Add approval
        transaction.approvals.push(owner);
        
        // Emit event
    emit!(TransactionApproved {
      transaction: transaction.key(),
      approver: owner,
      approvals_count: transaction.approvals.len() as u8,
      threshold: multisig.threshold,
     });

    Ok(())
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>, multisig_id: u64, _nonce: u64) -> Result<()> {
        let multisig = &ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        // Check if already executed
        require!(!transaction.did_execute, ErrorCode::AlreadyExecuted);

        // Check if enough approvals
        require!(
            transaction.approvals.len() >= multisig.threshold as usize,
            ErrorCode::NotEnoughApprovals
        );

        // Mark as executed
        transaction.did_execute = true;

        // Fix: Create proper seeds array
        let multisig_seeds: &[&[u8]] = &[
         b"multisig",
         &multisig_id.to_le_bytes(),
         &[ctx.bumps.multisig],
        ];

        // Build the instruction from stored data
      let instruction = anchor_lang::solana_program::instruction::Instruction {
      program_id: transaction.program_id,
      accounts: transaction.accounts.iter().map(|acc| {
          anchor_lang::solana_program::instruction::AccountMeta {
            pubkey: acc.pubkey,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
         }
       }).collect(),
       data: transaction.data.clone(),
    };

// Execute the instruction using Cross Program Invocation (CPI)
anchor_lang::solana_program::program::invoke_signed(
       &instruction,
        &ctx.remaining_accounts,
       &[multisig_seeds]
      )?;

        // Clear transaction data after execution to free up space
      transaction.data.clear();
      transaction.accounts.clear();

      // Emit event
    emit!(TransactionExecuted {
      transaction: transaction.key(),
      executor: ctx.accounts.executor.key(),
    });
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(multisig_id: u64)]
pub struct Initialize<'info> {
    #[account(
        init, 
        payer = creator, 
        space = 8 +                           // discriminator
                4 + (32 * MAX_OWNERS) +       // owners vec
                1 +                           // threshold
                32 +                          // creator
                8 +                           // multisig_id
                4 + (8 * MAX_STORED_NONCES),  // used_nonces vec
        seeds = [b"multisig", &multisig_id.to_le_bytes()],
        bump
    )]
    pub multisig: Account<'info, Multisig>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(multisig_id: u64, nonce: u64)]
pub struct CreateTransaction<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig", &multisig_id.to_le_bytes()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        init,
        payer = proposer,
        space = 8 +                           // discriminator
        32 +                          // multisig
        32 +                          // proposer  
        4 + (32 * MAX_OWNERS) +       // approvals vec
        1 +                           // did_execute
        8 +                           // nonce
        32 +                          // program_id
        4 + (65 * 10) +               // accounts vec (max 10 accounts, 65 bytes each)
        4 + 1024,                     // data vec (max 1024 bytes)
        seeds = [b"transaction", multisig.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub transaction: Account<'info, Transaction>,

    /// CHECK: Optional system nonce account
    pub nonce_account: Option<AccountInfo<'info>>,

    /// CHECK: Sysvar required by nonce account (optional)
    pub recent_blockhashes: Option<Sysvar<'info, RecentBlockhashes>>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(multisig_id: u64, nonce: u64)]
pub struct ApproveTransaction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"multisig", &multisig_id.to_le_bytes()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"transaction", multisig.key().as_ref(), &nonce.to_le_bytes()],
        bump,
    )]
    pub transaction: Account<'info, Transaction>,
}

// Fix: Remove the problematic remaining_accounts field from the struct
#[derive(Accounts)]
#[instruction(multisig_id: u64, nonce: u64)]
pub struct ExecuteTransaction<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,

    #[account(
        seeds = [b"multisig", &multisig_id.to_le_bytes()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"transaction", multisig.key().as_ref(), &nonce.to_le_bytes()],
        bump,
    )]
    pub transaction: Account<'info, Transaction>,
    // remaining_accounts are accessed via ctx.remaining_accounts in the function
}

#[account]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    pub creator: Pubkey,
    pub multisig_id: u64,
    pub used_nonces: Vec<u64>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[account]
pub struct Transaction {
    pub multisig: Pubkey,
    pub proposer: Pubkey,
    pub approvals: Vec<Pubkey>,
    pub did_execute: bool,
    pub nonce: u64,
    pub program_id: Pubkey,
    pub accounts: Vec<TransactionAccount>,
    pub data: Vec<u8>,
}

#[event]
pub struct TransactionCreated {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub proposer: Pubkey,
    pub nonce: u64,
}

#[event]
pub struct TransactionApproved {
    pub transaction: Pubkey,
    pub approver: Pubkey,
    pub approvals_count: u8,
    pub threshold: u8,
}

#[event]
pub struct TransactionExecuted {
    pub transaction: Pubkey,
    pub executor: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid threshold")]
    InvalidThreshold,
    #[msg("Duplicate owners")]
    DuplicateOwners,
    #[msg("No owner provided")]
    NoOwners,
    #[msg("The proposer is not an owner")]
    NotAnOwner,
    #[msg("Not an owner")]
    NotOwner,
    #[msg("Already approved")]
    AlreadyApproved,
    #[msg("Proposer is not the nonce authority")]
    InvalidNonceAuthority,
    #[msg("This nonce has already been used")]
    NonceAlreadyUsed,
    #[msg("Transaction already executed")]
    AlreadyExecuted,
    #[msg("Not enough approvals to execute")]
    NotEnoughApprovals,
    #[msg("Too many accounts in transaction")]
    TooManyAccounts,
    #[msg("Instruction data too large")]
    InstructionDataTooLarge,
    #[msg("Already an owner")]
    AlreadyAnOwner,
    #[msg("Too many owners")]
    TooManyOwners,
}