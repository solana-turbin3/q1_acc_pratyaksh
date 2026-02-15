use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::anchor::*; 
// use ephemeral_vrf_sdk::cpi::*;
// use ephemeral_vrf_sdk::consts::*; // Maybe for IDs?

use crate::state::UserAccount;

#[vrf]
#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", payer.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: The oracle queue (base layer or ephemeral)
    #[account(mut)]
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RequestRandomness<'info> {
    pub fn request_randomness(&mut self) -> Result<()> {
       
       let clock = Clock::get()?;
       let seed = clock
           .slot
           .checked_add(clock.unix_timestamp as u64)
           .unwrap()
           .to_le_bytes(); // Simple seed derivation
        
       let mut extra_seed = [0u8; 32];
       extra_seed[0..8].copy_from_slice(&seed);
       extra_seed[8..16].copy_from_slice(&self.payer.key().to_bytes()[0..8]);

       use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
       use ephemeral_vrf_sdk::types::SerializableAccountMeta;
       use anchor_lang::solana_program::program::invoke_signed;

       let params = RequestRandomnessParams {
           payer: self.payer.key(),
           oracle_queue: self.oracle_queue.key(),
           callback_program_id: crate::ID,
           callback_discriminator: crate::instruction::ConsumeRandomness::DISCRIMINATOR.to_vec(),
           accounts_metas: Some(vec![SerializableAccountMeta {
               pubkey: self.user_account.key(),
               is_signer: false,
               is_writable: true,
           }]),
           caller_seed: extra_seed,
           callback_args: None,
       };

       let ix = create_request_randomness_ix(params);

       let account_infos = [
           self.payer.to_account_info(),
           self.oracle_queue.to_account_info(),
           self.vrf_program.to_account_info(),
           self.program_identity.to_account_info(),
           self.system_program.to_account_info(),
           self.slot_hashes.to_account_info(),
       ];

       // Calculate seed for program_identity
       // Calculate seed for program_identity
       let (program_identity_pda, bump) = Pubkey::find_program_address(
           &[b"p-conf", crate::ID.as_ref()], 
           &ephemeral_vrf_sdk::id()
       );

       // Note: The #[vrf] macro checks this PDA, so we don't strictly need to check it again,
       // but we used to check: if program_identity_pda != self.program_identity.key() { struct_error }
       // Since we are invoking signed, we validly need the correct seeds.

       let seeds = &[
           b"p-conf", 
           crate::ID.as_ref(),
           &[bump]
       ];
       let signer_seeds = &[&seeds[..]];

       invoke_signed(
           &ix,
           &account_infos,
           signer_seeds
       )?;

       Ok(())
    }
}
