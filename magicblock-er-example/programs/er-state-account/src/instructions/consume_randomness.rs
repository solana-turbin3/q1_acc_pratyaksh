use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

use crate::state::UserAccount;

#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

impl<'info> ConsumeRandomness<'info> {
    pub fn consume_randomness(&mut self, randomness: [u8; 32]) -> Result<()> {
        let random_value = ephemeral_vrf_sdk::rnd::random_u64(&randomness);
        self.user_account.data = random_value;
        Ok(())
    }
}
