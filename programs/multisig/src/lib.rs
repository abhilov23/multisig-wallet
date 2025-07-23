use anchor_lang::prelude::*;

declare_id!("9ci6bSKQcGTEFGiDTRHacAf84jKuzwE3X5vHBWTDu5nb");

#[program]
pub mod multisig {
    use super::*;
    const MAX_OWNERS: usize = 10; //max user who can approve a transaction


    pub fn initialize(ctx: Context<Initialize>, owners: Vec<Pubkey>, threshold: u8) -> Result<()> {
       let multisig = &mut ctx.accounts.multisig;
       let creator = &ctx.accounts.creator;

       multisig.owners = owners;
       multisig.threshold = threshold;
       multisig.creator = creator.key(); //storing for future seed derivations

    if threshold > multisig.owners.len() as u8 {
           return Err(ErrorCode::InvalidThreshold.into());
    }
        
    if owners.is_empty() {
      return Err(ErrorCode::NoOwners.into());
    }


       //preventing duplicate owners
       let mut unique  = std::collections::HashSet::new();
         for owner in &owners {
            if !unique.insert(owner) {
                return Err(ErrorCode::DuplicateOwners.into());
            }
        }

        Ok(())
    }

    
    pub fn create_transaction(ctx: Context<CreateTransaction>, nonce: u8)-> Result<()>{
        let multisig = &mut ctx.accounts.multisig;
        let proposer = &ctx.accounts.proposer;
        let transaction = &mut ctx.accounts.transaction;

        require!(
            multisig.owners.contains(&proposer.key()),
            ErrorCode::NotAnOwner
        ); 

        // ✅ 2. Initialize transaction fields
         transaction.multisig = multisig.key();
         transaction.proposer = proposer.key();
         transaction.signers = vec![false; multisig.owners.len()];
         transaction.approvals = Vec![];
         transaction.did_execute = false;  

         Ok(())
    }

    pub fn approve_transaction(ctx: Context<ApproveTransaction>) -> Result<()> {
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

 
    // Add approval
    transaction.approvals.push(owner);

        Ok(())
    }
   

}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, 
    payer = creator, 
    space = 8 + 4 + (32 * MAX_OWNERS) + 1 + 32,
    seeds=[b"multisig", creator.key().as_ref()],
    bump
    )]
    pub multisig: Account<'info, Multisig>,   ///@Change the Multisig PDA to something more stable than this.
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,  //the person who creates the transaction

    #[account(
        seeds = [b"multisig", multisig.creator.as_ref()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>, //accessing the multisig data-account

    #[account(
        init,
        payer = proposer,
        space = 8 + 32 + 32 + 4 + MAX_OWNERS + 4 + (32 * MAX_OWNERS) + 1,
        seeds = [b"transaction", multisig.key().as_ref()],                  
        bump
    )]
    pub transaction: Account<'info, Transaction>, //accessing the transaction data-account

    pub system_program: Program<'info, System>,   //for initializing the transaction data-account
}


#[derive(Accounts)]
pub struct ApproveTransaction<'info>{
    #[account(mut)]
    pub owner: Signer<'info>,

     #[account(
        seeds = [b"multisig", multisig.creator.as_ref()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        seeds = [b"transaction", multisig.key().as_ref()],
        bump,
    )]
    pub transaction: Account<'info, Transaction>,
}




#[account]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u8, 
    pub creator: Pubkey, // the creator of the multisig
}

#[account]
pub struct Transaction{
    pub multisig: Pubkey, 
    pub proposer: Pubkey,
    pub signers: Vec<bool>,
    pub approvals: Vec<Pubkey>, // to keep track of who has approved the transaction
    pub did_execute: bool,
}


#[error_code]
pub enum ErrorCode {
    #[msg("Invalid threshold")]
    InvalidThreshold,
    #[msg("duplicate owners")]
    DuplicateOwners,
    #[msg("no owner provided")]
    NoOwners,
    #[msg("the proposer is not an owner")]
    NotAnOwner,
    #[msg("not an owner")]
    NotOwner,
    #[msg("already approved")]
    AlreadyApproved,
}
