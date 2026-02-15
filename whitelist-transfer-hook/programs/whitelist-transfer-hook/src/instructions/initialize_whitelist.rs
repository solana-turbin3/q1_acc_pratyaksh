use anchor_lang::prelude::*;

use crate::state::Whitelist;

#[derive(Accounts)]
pub struct InitializeWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        seeds = [b"whitelist"],
        bump,
        payer = admin,
        space = 8 + 32,
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeWhitelist<'info> {
    pub fn initialize_whitelist(&mut self, _bumps: &InitializeWhitelistBumps) -> Result<()> {
        self.whitelist.authority = self.admin.key();
        Ok(())
    }
}