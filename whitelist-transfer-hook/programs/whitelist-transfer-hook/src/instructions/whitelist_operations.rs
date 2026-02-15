use anchor_lang::prelude::*;

use crate::state::whitelist::{Whitelist, WhitelistEntry};

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct CreateWhitelistEntry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"whitelist"],
        bump,
        has_one = authority,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        init,
        payer = authority,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
        space = 8 + 32 + 1,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct CloseWhitelistEntry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"whitelist"],
        bump,
        has_one = authority,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        mut,
        close = authority,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateWhitelistEntry<'info> {
    pub fn create_whitelist_entry(&mut self, user: Pubkey, bump: u8) -> Result<()> {
        let whitelist_entry = &mut self.whitelist_entry;
        whitelist_entry.user = user;
        whitelist_entry.bump = bump;
        Ok(())
    }
}

impl<'info> CloseWhitelistEntry<'info> {
    pub fn close_whitelist_entry(&mut self, _user: Pubkey) -> Result<()> {
        Ok(())
    }
}