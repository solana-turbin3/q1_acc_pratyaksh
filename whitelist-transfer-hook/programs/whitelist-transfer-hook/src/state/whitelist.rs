use anchor_lang::prelude::*;

#[account]
pub struct Whitelist {
    pub authority: Pubkey,
}

#[account]
pub struct WhitelistEntry {
    pub user: Pubkey,
    pub bump: u8,
}