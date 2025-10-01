use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::{system_program, transaction::Transaction};

const GATEWAY_SEED: &[u8] = b"gateway";

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

    let (gateway_root_pda, _bump) = Pubkey::find_program_address(&[GATEWAY_SEED], &program_id);
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

    let gas_fee_amount: u64 = std::env::var("GAS_FEE_AMOUNT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_000);

    // Step 1: Call contract without gas payment
    println!("Step 1: Calling contract...");
    let call_contract_sig = call_contract(
        &rpc,
        &payer,
        program_id,
        &event_authority,
        &gateway_root_pda,
        &destination_chain,
        &destination_address,
        payload_hash,
        payload.clone(),
    )
    .await?;
    println!("Call contract tx: {}", call_contract_sig);

    // Step 2: Add native gas for the contract call
    println!("Step 2: Adding native gas...");
    let mut tx_hash = [0u8; 64];
    tx_hash.copy_from_slice(call_contract_sig.as_ref());
    let log_index = std::env::var("LOG_INDEX").unwrap_or_else(|_| "0.0".to_string()); // Using "0.0" as default log index

    // Validate the log_index format "x.y"
    if !validate_log_index_format(&log_index) {
        return Err(anyhow!(
            "LOG_INDEX must be in format 'x.y' where x and y are numbers, got: {}",
            log_index
        ));
    }
    let refund_address = payer.pubkey();

    let add_gas_sig = add_native_gas(
        &rpc,
        &payer,
        program_id,
        &event_authority,
        &gateway_root_pda,
        tx_hash,
        log_index,
        gas_fee_amount,
        refund_address,
    )
    .await?;
    println!("Add native gas tx: {}", add_gas_sig);

    println!("Successfully completed call_contract followed by add_native_gas!");
    println!("Contract call tx hash: {:?}", tx_hash);
    println!("Gas amount added: {}", gas_fee_amount);

    Ok(())
}

async fn call_contract(
    rpc: &RpcClient,
    payer: &solana_sdk::signature::Keypair,
    program_id: Pubkey,
    event_authority: &Pubkey,
    gateway_root_pda: &Pubkey,
    destination_chain: &str,
    destination_contract_address: &str,
    payload_hash: [u8; 32],
    payload: Vec<u8>,
) -> Result<solana_sdk::signature::Signature> {
    let mut data = Vec::new();
    data.extend_from_slice(&anchor_method_discriminator("call_contract"));
    serialize_string(destination_chain, &mut data);
    serialize_string(destination_contract_address, &mut data);
    data.extend_from_slice(&payload_hash);

    // Serialize payload as Vec<u8>
    data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&payload);

    let accounts = vec![
        AccountMeta::new_readonly(payer.pubkey(), false), // calling_program
        AccountMeta::new_readonly(payer.pubkey(), false), // signing_pda (using payer as dummy)
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    send_ix(rpc, payer, &[ix]).await
}

fn validate_log_index_format(log_index: &str) -> bool {
    let parts: Vec<&str> = log_index.split('.').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[0].parse::<u64>().is_ok() && parts[1].parse::<u64>().is_ok()
}

async fn add_native_gas(
    rpc: &RpcClient,
    payer: &solana_sdk::signature::Keypair,
    program_id: Pubkey,
    event_authority: &Pubkey,
    config_pda: &Pubkey,
    tx_hash: [u8; 64],
    log_index: String,
    gas_fee_amount: u64,
    refund_address: Pubkey,
) -> Result<solana_sdk::signature::Signature> {
    let mut data = Vec::new();
    data.extend_from_slice(&anchor_method_discriminator("add_native_gas"));
    data.extend_from_slice(&tx_hash);
    serialize_string(&log_index, &mut data);
    data.extend_from_slice(&gas_fee_amount.to_le_bytes());
    data.extend_from_slice(refund_address.as_ref());

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),        // sender
        AccountMeta::new_readonly(*config_pda, false), // config_pda
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    send_ix(rpc, payer, &[ix]).await
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
