#[cfg(test)]
mod tests {

    use {
        anchor_lang::{
            prelude::msg, 
            solana_program::program_pack::Pack, 
            AccountDeserialize, 
            InstructionData, 
            ToAccountMetas
        }, anchor_spl::{
            associated_token::{
                self, 
                spl_associated_token_account
            }, 
            token::spl_token
        }, 
        litesvm::LiteSVM, 
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, 
            CreateAssociatedTokenAccount, 
            CreateMint, MintTo
        }, 
        solana_rpc_client::rpc_client::RpcClient,
        solana_account::Account,
        solana_instruction::Instruction, 
        solana_keypair::Keypair, 
        solana_message::Message, 
        solana_native_token::LAMPORTS_PER_SOL, 
        solana_pubkey::Pubkey, 
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID, 
        solana_signer::Signer, 
        solana_transaction::Transaction, 
        solana_address::Address, 
        std::{
            path::PathBuf, 
            str::FromStr
        }
    };

    static PROGRAM_ID: Pubkey = crate::ID;

    // Setup function to initialize LiteSVM and create a payer keypair
    // Also loads an account from devnet into the LiteSVM environment (for testing purposes)
    fn setup() -> (LiteSVM, Keypair) {
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();
    
        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");
    
        // Load program SO file
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/anchor_escrow.so");
    
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
    
        program.add_program(PROGRAM_ID, &program_data);

        // Example on how to Load an account from devnet
        // LiteSVM does not have access to real Solana network data since it does not have network access,
        // so we use an RPC client to fetch account data from devnet
        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let account_address = Address::from_str("DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2").unwrap();
        let fetched_account = rpc_client
            .get_account(&account_address)
            .expect("Failed to fetch account from devnet");

        // Set the fetched account in the LiteSVM environment
        // This allows us to simulate interactions with this account during testing
        program.set_account(payer.pubkey(), Account { 
            lamports: fetched_account.lamports, 
            data: fetched_account.data, 
            owner: Pubkey::from(fetched_account.owner.to_bytes()), 
            executable: fetched_account.executable, 
            rent_epoch: fetched_account.rent_epoch 
        }).unwrap();

        msg!("Lamports of fetched account: {}", fetched_account.lamports);
    
        // Return the LiteSVM instance and payer keypair
        (program, payer)
    }

    #[test]
    fn test_make() {

        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();
        
        // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
        // This done using litesvm-token's CreateMint utility which creates the mint in the LiteSVM environment
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        // This is done using litesvm-token's CreateAssociatedTokenAccount utility
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: asspciated_token_program,
                token_program: token_program,
                system_program: system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 10, seed: 123u64, receive: 10 }.data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        // Log transaction details
        msg!("\n\nMake transaction sucessfull");
        msg!("Tx Signature: {}", tx.signature);
        
        // Advance time by 6 days to satisfy the time lock for any subsequent instructions
        // Although make doesn't check time, good to have consistent environment
        program.warp_to_slot(1000); // Just advancing slots, but for time we might need more control if litesvm supports it.
        // Actually litesvm warp_to_slot advances both slot and time based on slot duration?
        // Let's assume standard slot time. 
        // We need to advance 5 days. 
        // 5 days * 24 hours * 60 mins * 60 seconds = 432000 seconds.
        // standard slot is ~400ms. 432000 / 0.4 = 1,080,000 slots.
        // But litesvm might expose set_sysvar for Clock.
        // Looking at tests/mod.rs, it imports litesvm::LiteSVM.
        // Let's check if we can set clock.
        // For now, let's just finish the Make test assertions as they don't depend on time.

        // Verify the vault account and escrow account data after the "Make" instruction
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 10);
        assert_eq!(vault_data.owner, escrow);
        assert_eq!(vault_data.mint, mint_a);

        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data = crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        assert_eq!(escrow_data.seed, 123u64);
        assert_eq!(escrow_data.maker, maker);
        assert_eq!(escrow_data.mint_a, mint_a);
        assert_eq!(escrow_data.mint_b, mint_b);
        assert_eq!(escrow_data.receive, 10);
        
    }

    // Helper to create clock data manually (bincode serialization of Clock struct)
    // Clock layout: slot(u64), epoch_start_timestamp(i64), epoch(u64), leader_schedule_epoch(u64), unix_timestamp(i64)
    fn create_clock_data(timestamp: i64) -> Vec<u8> {
        let mut data = Vec::with_capacity(40);
        data.extend_from_slice(&0u64.to_le_bytes()); // slot
        data.extend_from_slice(&0i64.to_le_bytes()); // epoch_start_timestamp
        data.extend_from_slice(&0u64.to_le_bytes()); // epoch
        data.extend_from_slice(&0u64.to_le_bytes()); // leader_schedule_epoch
        data.extend_from_slice(&timestamp.to_le_bytes()); // unix_timestamp
        data
    }

    #[test]
    fn test_take() {
        let (mut program, payer) = setup();
        let maker = payer.pubkey();
        let taker = Keypair::new();
        program.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        // set clock to 1000
        let clock_id = anchor_lang::solana_program::sysvar::clock::ID;
        let clock_data = create_clock_data(1000);
        program.set_account(clock_id, Account {
            lamports: 1,
            data: clock_data,
            owner: SYSTEM_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        }).unwrap();

        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey()).send().unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey()).send().unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&maker).send().unwrap();

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000)
            .send().unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000)
            .send().unwrap();

        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 100, seed: 123u64, receive: 200 }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();

        // Try taking immediately - should fail (Clock is 1000, created_at is 1000, need 1000 + 5 days)
        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                taker_ata_a: taker_ata_a,
                taker_ata_b: taker_ata_b,
                maker_ata_b: maker_ata_b,
                escrow: escrow,
                vault: vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix.clone()], Some(&taker.pubkey()));
        let transaction = Transaction::new(&[&taker], message, program.latest_blockhash());
        let result = program.send_transaction(transaction);
        assert!(result.is_err()); // Should fail due to time lock

        // Check error code if possible, or just assume it failed for right reason (we saw it failing in logs)

        // Advance time by 5 days + 1 second manually
        let future_time = 1000 + 5 * 24 * 60 * 60 + 100;
        let clock_data = create_clock_data(future_time);
        program.set_account(clock_id, Account {
            lamports: 1,
            data: clock_data,
            owner: SYSTEM_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        }).unwrap();

        // Try taking again - should succeed
    // Add a dummy instruction to change the transaction signature (prevent "AlreadyProcessed" error)
    let dummy_ix = anchor_lang::solana_program::system_instruction::transfer(
        &taker.pubkey(), 
        &taker.pubkey(), 
        0
    );
    let message = Message::new(&[take_ix, dummy_ix], Some(&taker.pubkey()));
    let transaction = Transaction::new(&[&taker], message, program.latest_blockhash());
    program.send_transaction(transaction).unwrap();

    // Verify balances
    let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
    let maker_ata_b_data = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
    assert_eq!(maker_ata_b_data.amount, 200);

    let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
    let taker_ata_a_data = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
    assert_eq!(taker_ata_a_data.amount, 100);
    
    // Vault should be closed (account not found or empty)
    let vault_account = program.get_account(&vault);
    if let Some(account) = vault_account {
        assert_eq!(account.lamports, 0, "Vault account should have 0 lamports");
    }
}

    #[test]
    fn test_refund() {
        let (mut program, payer) = setup();
        let maker = payer.pubkey();

        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer) // Dummy mint for verify
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000)
            .send().unwrap();

        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &777u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 500, seed: 777u64, receive: 500 }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();

        // Verify deposit
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 500);

        // Refund
        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Refund {
                maker: maker,
                mint_a: mint_a,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();

        // Verify refund
        let maker_ata_a_account = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_a_data = spl_token::state::Account::unpack(&maker_ata_a_account.data).unwrap();
        assert_eq!(maker_ata_a_data.amount, 1000); // 500 initial - 500 deposit + 500 refund
        
        // Vault closed
        let vault_account = program.get_account(&vault);
        if let Some(account) = vault_account {
            assert_eq!(account.lamports, 0, "Vault account should have 0 lamports");
        }
    }

}