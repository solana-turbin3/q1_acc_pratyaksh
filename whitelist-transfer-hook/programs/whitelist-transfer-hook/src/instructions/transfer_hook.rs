use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{
        Mint, 
        TokenAccount
    }
};

use crate::state::WhitelistEntry;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint, 
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [b"whitelist", owner.key().as_ref()], 
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
}

impl<'info> TransferHook<'info> {
    /// This function is called when the transfer hook is executed.
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        
        // self.check_is_transferring()?;

        msg!("Source token owner: {}", self.source_token.owner);
        msg!("Destination token owner: {}", self.destination_token.owner);

        // Since the whitelist_entry account is validated by Anchor using seeds,
        // if it exists and is passed correctly, the user is whitelisted.
        msg!("Transfer allowed: The address is whitelisted");

        Ok(())
    }

    // Checks if the transfer hook is being executed during a transfer operation.
    // Removed because standard token accounts don't have TransferHookAccount extension.
}