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

    // let mut tx_hash = [0u8; 64];
    // tx_hash.copy_from_slice(
    //     "4sFqQ9mjg5d61BBnuWeT5spHkM9jr9cAfn6ghgMBMYEK89hJj5gFLCqap3o4Z6779rcmA6ziXBGxU6rJNg3sKVf6"
    //         .as_bytes(),
    // );

    let mut tx_hash = [
        96, 234, 53, 170, 139, 128, 159, 106, 180, 136, 227, 149, 236, 95, 149, 154, 21, 245, 188,
        217, 46, 43, 133, 179, 63, 169, 153, 86, 49, 219, 100, 18, 107, 141, 155, 116, 138, 75,
        118, 176, 2, 8, 194, 253, 99, 217, 148, 149, 250, 91, 31, 172, 138, 185, 63, 56, 152, 241,
        121, 164, 27, 139, 23, 12,
    ];

    let d1 = Sha256::digest(b"refund-tx-hash-part-1");
    let d2 = Sha256::digest(b"refund-tx-hash-part-2");
    tx_hash[..32].copy_from_slice(&d1);
    tx_hash[32..].copy_from_slice(&d2);

    let ix_index: u8 = std::env::var("IX_INDEX")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(1);

    let event_ix_index: u8 = std::env::var("EVENT_IX_INDEX")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(1);

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
        ix_index,
        event_ix_index,
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

fn build_refund_native_fees_ix(
    program_id: &Pubkey,
    config_pda: &Pubkey,
    receiver: &Pubkey,
    event_authority: &Pubkey,
    tx_hash: [u8; 64],
    ix_index: u8,
    event_ix_index: u8,
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
    data.push(ix_index);
    data.push(event_ix_index);
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
