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
            .unwrap_or_else(|_| "CJ9f8WFdm3q38pmg426xQf7uum7RqvrmS9R58usHwNX7".to_string()),
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

    let message_id =
        std::env::var("MESSAGE_ID").unwrap_or_else(|_| "3Yoe1V1qMFERAVXadHkrnXWQ2STa7Yd8rydoWxouXQrpwtDZGpuVPdmdJSA9HiNQi91aFP5EumZrvAqZcQa84Ens-2.1".to_string());

    let amount: u64 = std::env::var("REFUND_AMOUNT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(500);

    let receiver = payer.pubkey();

    let ix = build_refund_native_fees_ix(
        &program_id,
        &config_pda,
        &receiver,
        &event_authority,
        message_id.clone(),
        amount,
    )?;

    let sig = send_ix(&rpc, &payer, &[ix]).await?;
    println!("Sent refund_native_fees tx: {}", sig);
    println!("Message ID: {}", message_id);
    println!("Refund amount: {}", amount);

    let refunded_disc = anchor_event_struct_discriminator("GasRefundedEvent");
    println!("GasRefundedEvent discriminator: {:#04x?}", refunded_disc);

    Ok(())
}

fn build_refund_native_fees_ix(
    program_id: &Pubkey,
    config_pda: &Pubkey,
    receiver: &Pubkey,
    event_authority: &Pubkey,
    message_id: String,
    amount: u64,
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

    // Serialize message_id as String
    let message_id_bytes = message_id.as_bytes();
    data.extend_from_slice(&(message_id_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(message_id_bytes);

    data.extend_from_slice(&amount.to_le_bytes());

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
