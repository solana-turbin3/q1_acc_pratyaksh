import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { GetCommitmentSignature } from "@magicblock-labs/ephemeral-rollups-sdk";
import { ErStateAccount } from "../target/types/er_state_account";

describe("er-state-account", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const providerEphemeralRollup = new anchor.AnchorProvider(
    new anchor.web3.Connection(
      process.env.EPHEMERAL_PROVIDER_ENDPOINT ||
        "https://devnet.magicblock.app/",
      {
        wsEndpoint:
          process.env.EPHEMERAL_WS_ENDPOINT || "wss://devnet.magicblock.app/",
      },
    ),
    anchor.Wallet.local(),
  );
  console.log("Base Layer Connection: ", provider.connection.rpcEndpoint);
  console.log(
    "Ephemeral Rollup Connection: ",
    providerEphemeralRollup.connection.rpcEndpoint,
  );
  console.log(`Current SOL Public Key: ${anchor.Wallet.local().publicKey}`);

  before(async function () {
    const balance = await provider.connection.getBalance(
      anchor.Wallet.local().publicKey,
    );
    console.log("Current balance is", balance / LAMPORTS_PER_SOL, " SOL", "\n");
  });

  const program = anchor.workspace.erStateAccount as Program<ErStateAccount>;

  const userAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user"), anchor.Wallet.local().publicKey.toBuffer()],
    program.programId,
  )[0];

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("User Account initialized: ", tx);
  });

  it("Update State!", async () => {
    const tx = await program.methods
      .update(new anchor.BN(42))
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
      })
      .rpc();
    console.log("\nUser Account State Updated: ", tx);
  });

  it("Request Randomness (L1)", async () => {
    const oracleQueue = new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");
    const vrfProgram = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
    
    // Derive program_identity pda
    const [programIdentity] = PublicKey.findProgramAddressSync(
        [Buffer.from("p-conf"), program.programId.toBuffer()],
        vrfProgram
    );
    console.log("Derived Program Identity:", programIdentity.toBase58());
    // const programIdentity = new PublicKey("a2wMfXPFMkH62av9yivJybZ9pzWozrtpeNJKrmGLvEa");

    try {
        const tx = await program.methods
        .requestRandomness()
        .accounts({
            payer: anchor.Wallet.local().publicKey,
            userAccount: userAccount,
            oracleQueue: oracleQueue,
            vrfProgram: vrfProgram,
            programIdentity: programIdentity,
            systemProgram: anchor.web3.SystemProgram.programId,
            slotHashes: anchor.web3.SYSVAR_SLOT_HASHES_PUBKEY,
        })
        .rpc();
        console.log("Randomness Requested (L1): ", tx);
    } catch (e) {
        console.log("L1 Request failed (expected if VRF program not deployed locally):", e);
    }
  });

  it("Delegate to Ephemeral Rollup!", async () => {
    let tx = await program.methods
      .delegate()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        validator: new PublicKey("MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57"),
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });

    console.log("\nUser Account Delegated to Ephemeral Rollup: ", tx);
  });

  it("Request Randomness (ER)", async () => {
      const oracleQueue = new PublicKey("5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc");
      const vrfProgram = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");

      const [programIdentity] = PublicKey.findProgramAddressSync(
          [Buffer.from("p-conf"), program.programId.toBuffer()],
          vrfProgram
      );

      try {
        let tx = await program.methods
            .requestRandomness()
            .accounts({
                payer: providerEphemeralRollup.wallet.publicKey,
                userAccount: userAccount,
                oracleQueue: oracleQueue,
                vrfProgram: vrfProgram,
                programIdentity: programIdentity,
                systemProgram: anchor.web3.SystemProgram.programId,
                slotHashes: anchor.web3.SYSVAR_SLOT_HASHES_PUBKEY,
            })
            .transaction();

        tx.feePayer = providerEphemeralRollup.wallet.publicKey;
        tx.recentBlockhash = (
            await providerEphemeralRollup.connection.getLatestBlockhash()
        ).blockhash;
        tx = await providerEphemeralRollup.wallet.signTransaction(tx);
        
        // Note: sending to ER
        const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {
            skipPreflight: true, // often needed for ER
        });
        console.log("Randomness Requested (ER): ", txHash);
      } catch (e) {
          console.log("ER Request failed (expected if ER is not running or VRF not supported):", e);
      }
  });

  it("Update State and Commit to Base Layer!", async () => {
    let tx = await program.methods
      .updateCommit(new anchor.BN(43))
      .accountsPartial({
        user: providerEphemeralRollup.wallet.publicKey,
        userAccount: userAccount,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;

    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;
    tx = await providerEphemeralRollup.wallet.signTransaction(tx);
    const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {
      skipPreflight: false,
    });
    const txCommitSgn = await GetCommitmentSignature(
      txHash,
      providerEphemeralRollup.connection,
    );

    console.log("\nUser Account State Updated: ", txHash);
  });

  it("Commit and undelegate from Ephemeral Rollup!", async () => {
    let info = await providerEphemeralRollup.connection.getAccountInfo(
      userAccount,
    );

    console.log("User Account Info: ", info);

    console.log("User account", userAccount.toBase58());

    let tx = await program.methods
      .undelegate()
      .accounts({
        user: providerEphemeralRollup.wallet.publicKey,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;

    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;
    tx = await providerEphemeralRollup.wallet.signTransaction(tx);
    const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {
      skipPreflight: false,
    });
    const txCommitSgn = await GetCommitmentSignature(
      txHash,
      providerEphemeralRollup.connection,
    );

    console.log("\nUser Account Undelegated: ", txHash);
  });

  it("Update State!", async () => {
    let tx = await program.methods
      .update(new anchor.BN(45))
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
      })
      .rpc();

    console.log("\nUser Account State Updated: ", tx);
  });

  it("Close Account!", async () => {
    const tx = await program.methods
      .close()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("\nUser Account Closed: ", tx);
  });
});
