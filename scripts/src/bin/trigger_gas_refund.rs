use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::transaction::Transaction;

const CONFIG_SEED: &[u8] = b"config";

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

fn anchor_method_discriminator(name: &str) -> [u8; 8] {
    // Anchor method discriminator = sha256("global:<method_name>")[..8]
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}"));
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());

    // Gas service program ID
    let program_id = Pubkey::from_str(
        &std::env::var("GAS_PROGRAM_ID")
            .unwrap_or_else(|_| "H9XpBVCnYxr7cHd66nqtD8RSTrKY6JC32XVu2zT2kBmP".to_string()),
    )?;

    let payer_path = std::env::var("PAYER")
        .unwrap_or_else(|_| "/Users/nikos/.config/solana/id.json".to_string());
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let (derived_config_pda, _bump) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);

    let config_pda = match rpc.get_account(&derived_config_pda).await {
        Ok(_) => derived_config_pda,
        Err(_) => payer.pubkey(),
    };

    let mut tx_hash = [0u8; 64];
    let d1 = Sha256::digest(b"refund-tx-hash-part-1");
    let d2 = Sha256::digest(b"refund-tx-hash-part-2");
    tx_hash[..32].copy_from_slice(&d1);
    tx_hash[32..].copy_from_slice(&d2);

    let log_index: String = std::env::var("LOG_INDEX").unwrap_or_else(|_| "1.0".to_string());

    // Validate the log_index format "x.y"
    if !validate_log_index_format(&log_index) {
        return Err(anyhow!(
            "LOG_INDEX must be in format 'x.y' where x and y are numbers, got: {}",
            log_index
        ));
    }

    let fees: u64 = std::env::var("REFUND_FEES")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(500);

    let receiver = payer.pubkey();

    let ix = build_refund_native_fees_ix(
        &program_id,
        &config_pda,
        &receiver,
        &event_authority,
        tx_hash,
        log_index,
        fees,
    )?;

    let sig = send_ix(&rpc, &payer, &[ix]).await?;
    println!("Sent refund_native_fees tx: {}", sig);

    println!("EVENT_IX_TAG_LE: {:#04x?}", EVENT_IX_TAG_LE);
    let refunded_disc = anchor_event_struct_discriminator("NativeGasRefundedEvent");
    println!(
        "NativeGasRefundedEvent discriminator: {:#04x?}",
        refunded_disc
    );

    Ok(())
}

fn serialize_string(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn validate_log_index_format(log_index: &str) -> bool {
    let parts: Vec<&str> = log_index.split('.').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[0].parse::<u64>().is_ok() && parts[1].parse::<u64>().is_ok()
}

fn build_refund_native_fees_ix(
    program_id: &Pubkey,
    config_pda: &Pubkey,
    receiver: &Pubkey,
    event_authority: &Pubkey,
    tx_hash: [u8; 64],
    log_index: String,
    fees: u64,
) -> Result<Instruction> {
    let accounts = vec![
        AccountMeta::new_readonly(*config_pda, false),
        AccountMeta::new_readonly(*receiver, false),
        AccountMeta::new_readonly(*event_authority, false),
        AccountMeta::new_readonly(*program_id, false),
    ];

    let disc = anchor_method_discriminator("refund_native_fees");
    let mut data = Vec::new();
    data.extend_from_slice(&disc);
    data.extend_from_slice(&tx_hash);
    serialize_string(&log_index, &mut data);
    data.extend_from_slice(&fees.to_le_bytes());

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
