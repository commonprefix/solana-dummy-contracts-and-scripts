use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
// use solana_client::rpc_response::Response as RpcResponse;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::{system_program, transaction::Transaction};

const CONFIG_SEED: &[u8] = b"config";

fn anchor_method_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}"));
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

fn serialize_string(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
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

    let destination_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let destination_address = std::env::var("DEST_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());

    let payload: Vec<u8> = vec![1, 2, 3];
    let payload_hash = {
        let digest = Sha256::digest(&payload);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&digest[..32]);
        arr
    };

    let refund_address = payer.pubkey();
    let gas_fee_amount: u64 = std::env::var("GAS_FEE_AMOUNT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_000);

    // Build pay_native_for_contract_call
    let mut data = Vec::with_capacity(8 + 256);
    data.extend_from_slice(&anchor_method_discriminator("pay_native_for_contract_call"));
    serialize_string(&destination_chain, &mut data);
    serialize_string(&destination_address, &mut data);
    data.extend_from_slice(&payload_hash);
    data.extend_from_slice(refund_address.as_ref());
    data.extend_from_slice(&gas_fee_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    let sig = send_ix(&rpc, &payer, &[ix]).await?;
    println!("Sent pay_native_for_contract_call tx: {}", sig);

    Ok(())
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
