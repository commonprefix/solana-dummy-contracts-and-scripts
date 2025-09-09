use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_response::Response as RpcResponse;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::{system_program, transaction::Transaction};

const DISC_INITIALIZE: [u8; 1] = [0]; // GasServiceDiscriminators::INITIALIZE
const DISC_ADD_NATIVE_GAS: [u8; 2] = [3, 1]; // GasServiceDiscriminators::NATIVE_ADD_GAS
const CONFIG_SEED: &[u8] = b"gas-service";

// Anchor event CPI instruction tag (little-endian)
const EVENT_IX_TAG_LE: [u8; 8] = [0xe4, 0x45, 0xa5, 0x2e, 0x51, 0xcb, 0x9a, 0x1d];

fn anchor_event_struct_discriminator(type_name: &str) -> [u8; 8] {
    // Anchor event struct discriminator = sha256("event:<TypeName>")[..8]
    let mut hasher = Sha256::new();
    hasher.update(format!("event:{type_name}"));
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let program_id = Pubkey::from_str(
        &std::env::var("PROGRAM_ID")
            .unwrap_or_else(|_| "7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc".to_string()),
    )?;

    let payer_path = std::env::var("PAYER")
        .unwrap_or_else(|_| "/Users/nikos/.config/solana/id.json".to_string());
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let (config_pda, _bump) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);

    let needs_init = rpc.get_account(&config_pda).await.is_err();
    if needs_init {
        let ix = build_initialize_ix(&program_id, &payer.pubkey(), &config_pda)?;
        send_ix(&rpc, &payer, &[ix]).await?;
        println!("Initialized config PDA: {}", config_pda);
    } else {
        println!("Config PDA already exists: {}", config_pda);
    }

    let mut tx_hash = [0u8; 64];
    let d1 = Sha256::digest(b"tx-hash-part-1");
    let d2 = Sha256::digest(b"tx-hash-part-2");
    tx_hash[..32].copy_from_slice(&d1);
    tx_hash[32..].copy_from_slice(&d2);

    let log_index: u64 = std::env::var("LOG_INDEX")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);

    let gas_fee_amount: u64 = std::env::var("GAS_FEE_AMOUNT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_000);

    let refund_address = payer.pubkey();

    let ix = build_add_native_gas_ix(
        &program_id,
        &payer.pubkey(),
        &config_pda,
        &event_authority,
        tx_hash,
        log_index,
        gas_fee_amount,
        refund_address,
    )?;

    let sig = send_ix(&rpc, &payer, &[ix]).await?;
    println!("Sent add_native_gas tx: {}", sig);

    println!("EVENT_IX_TAG_LE: {:#04x?}", EVENT_IX_TAG_LE);
    let spl_gas_paid_disc = anchor_event_struct_discriminator("SplGasPaidForContractCallEvent");
    println!(
        "SplGasPaidForContractCallEvent discriminator: {:#04x?}",
        spl_gas_paid_disc
    );

    Ok(())
}

fn build_initialize_ix(
    program_id: &Pubkey,
    payer: &Pubkey,
    config_pda: &Pubkey,
) -> Result<Instruction> {
    // Accounts: payer (mut signer), operator (signer), config_pda (init, mut), system_program
    // Here we reuse payer as operator for simplicity.
    let accounts = vec![
        AccountMeta::new(*payer, true),          // payer
        AccountMeta::new_readonly(*payer, true), // operator
        AccountMeta::new(*config_pda, false),    // config_pda (writable)
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = DISC_INITIALIZE.to_vec(); // no args
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

fn build_add_native_gas_ix(
    program_id: &Pubkey,
    sender: &Pubkey,
    config_pda: &Pubkey,
    event_authority: &Pubkey,
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
) -> Result<Instruction> {
    let accounts = vec![
        AccountMeta::new(*sender, true),                        // sender
        AccountMeta::new(*config_pda, false),                   // config_pda (writable)
        AccountMeta::new_readonly(system_program::id(), false), // system_program
        AccountMeta::new_readonly(*event_authority, false),     // #[event_cpi]
        AccountMeta::new_readonly(*program_id, false),          // #[event_cpi]
    ];

    // Data layout: [3,1] + [u8;64] + u64 + u64 + Pubkey
    let mut data = Vec::with_capacity(2 + 64 + 8 + 8 + 32);
    data.extend_from_slice(&DISC_ADD_NATIVE_GAS);
    data.extend_from_slice(&tx_hash);
    data.extend_from_slice(&log_index.to_le_bytes());
    data.extend_from_slice(&gas_fee_amount.to_le_bytes());
    data.extend_from_slice(refund_address.as_ref());

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

async fn send_ix(
    rpc: &RpcClient,
    payer: &solana_sdk::signature::Keypair,
    ixs: &[Instruction],
) -> Result<solana_sdk::signature::Signature> {
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(ixs, Some(&payer.pubkey()));
    tx.sign(&[payer], recent_blockhash);
    let sig = rpc.send_and_confirm_transaction(&tx).await?;
    Ok(sig)
}
