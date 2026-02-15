use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::{
    token_interface::{
        TokenInterface,
    },
    token_2022::{
        self,
        spl_token_2022::{
            extension::{
                ExtensionType,
                transfer_hook,
            },
            state::Mint as MintState,
        },
    },
};

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    // We strictly use Signer here because we are creating the account from scratch
    #[account(mut)]
    pub mint: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> CreateToken<'info> {
    pub fn create_token(
        &mut self, 
        _uri: String, 
        _name: String, 
        _symbol: String,
        _bumps: &CreateTokenBumps,
    ) -> Result<()> {
        
        let space = ExtensionType::try_calculate_account_len::<MintState>(&[
            ExtensionType::TransferHook,
        ])?;

        let lamports = (Rent::get()?).minimum_balance(space);

        // 1. Create Account
        system_program::create_account(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::CreateAccount {
                    from: self.payer.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            lamports,
            space as u64,
            &self.token_program.key(),
        )?;

        // 2. Initialize Transfer Hook Extension
        // We use solana_program::program::invoke for this because anchor_spl types might be tricky to construct cpi for unknown extensions
        // But we can use the instruction builder from spl_token_2022
        
        let ix = transfer_hook::instruction::initialize(
            &self.token_program.key(),
            &self.mint.key(),
            Some(self.payer.key()),
            Some(crate::ID), // Transfer Hook Program ID
        )?;
        
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.token_program.to_account_info(),
                self.mint.to_account_info(),
            ],
        )?;

        // 3. Initialize Mint
        token_2022::initialize_mint2(
            CpiContext::new(
                self.token_program.to_account_info(),
                token_2022::InitializeMint2 {
                    mint: self.mint.to_account_info(),
                },
            ),
            9,
            &self.payer.key(),
            Some(&self.payer.key()),
        )?;

        Ok(())
    }
}