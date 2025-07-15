use anchor_lang::prelude::*;

declare_id!("9ci6bSKQcGTEFGiDTRHacAf84jKuzwE3X5vHBWTDu5nb");

#[program]
pub mod multisig {
    use super::*;

    //Parameters:
    // 1. ctx: Context<Initialize>
    // 2. owners: vector of public keys
    // 3. threshold: total number of approvals we need

    pub fn initialize(ctx: Context<Initialize>, owners: Vec<Pubkey>, threshold: u8) -> Result<()> {
       let multisig = &mut ctx.accounts.multisig;
       multisig.owners = owners.clone();
       multisig.threshold = threshold;
       
       if threshold > multisig.owners.len() as u8 {
           return Err(ErrorCode::InvalidThreshold.into());
       }
       //preventing duplicate owners
       for i in 0..multisig.owners.len() {
           for j in i+1..multisig.owners.len() {
               if multisig.owners[i] == multisig.owners[j] {
                   return Err(ErrorCode::DuplicateOwners.into());
               }
           }
       }

       if owners.is_empty() {
           return Err(ErrorCode::NoOwners.into());
       }
        Ok(())
    }

    
    pub fn create_transaction(ctx: Context<CreateTransaction>, nonce: u8)-> Result(()){
        let multisig = &mut ctx.accounts.multisig;
        let proposer = &ctx.accounts.proposer;
        let transaction = &mut ctx.accounts.transaction;

        //let's verify  if the proposer is one of the owner
        require!(
            multisig.owners.contains(&proposer.key()),
            ErrorCode::NotAnOwner
        ); 

        // ✅ 2. Initialize transaction fields
         transaction.multisig = multisig.key();
         transaction.proposer = proposer.key();
         transaction.signers = vec![false; multisig.owners.len()];
         transaction.did_execute = false;  
         Ok(())
    }
   

}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, 
    payer = user, 
    space = 8 + 32 + 32 + 4 + 10 + 1,
    seeds=[b"multisig", user.key().as_ref()],
    bump
    )]
    pub multisig: Account<'info, Multisig>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct CreateTransaction<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,  //the person who creates the transaction

    #[account(
        seeds = [b"multisig", proposer.key().as_ref()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>, //accessing the multisig data-account

    #[account(
        init,
        payer = proposer,
        space = <calculate>,
        seeds = [b"transaction", multisig.key().as_ref(), &[nonce]],
        bump
    )]
    pub transaction: Account<'info, Transaction>, //accessing the transaction data-account

    pub system_program: Program<'info, System>,   //for initializing the transaction data-account
}





#[account]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u8, 
}

#[account]
pub struct Transaction{
    pub multisig: Pubkey, 
    pub proposer: Pubkey,
    pub signers: Vec<bool>,
    pub did_execute: bool
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
    NotAnOwner
}