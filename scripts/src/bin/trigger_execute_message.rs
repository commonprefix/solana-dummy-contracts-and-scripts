use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::keccak;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::{system_program, transaction::Transaction};

fn anchor_method_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}"));
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

fn put_string(s: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
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

    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);

    // Get the message details from environment variables or use defaults
    let cc_chain = std::env::var("SRC_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let cc_id = std::env::var("SRC_ID").unwrap_or_else(|_| "0xabc".to_string());
    let src_address = std::env::var("SRC_ADDR").unwrap_or_else(|_| "0xdead".to_string());
    let dst_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "solana".to_string());
    let dst_address = std::env::var("DEST_ADDR").unwrap_or_else(|_| payer.pubkey().to_string());

    // Compute command_id for the message
    let command_id = keccak::hashv(&[cc_chain.as_bytes(), b"-", cc_id.as_bytes()]).0;

    // Generate a dummy payload hash for testing
    let payload_hash = keccak::hashv(&[b"test_payload"]).0;

    // Build execute_message instruction data
    let mut data = Vec::new();
    data.extend_from_slice(&anchor_method_discriminator("execute_message"));

    // Add command_id
    data.extend_from_slice(&command_id);

    // Add string parameters
    put_string(&cc_chain, &mut data); // source_chain
    put_string(&cc_id, &mut data); // cc_id
    put_string(&src_address, &mut data); // source_address
    put_string(&dst_chain, &mut data); // destination_chain
    put_string(&dst_address, &mut data); // destination_address

    // Add payload_hash
    data.extend_from_slice(&payload_hash);

    // Accounts for ExecuteMessage
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true), // funder
        AccountMeta::new_readonly(system_program::id(), false),
        // Event CPI injected
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    // Execute the instruction
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    let sig = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Sent execute_message tx: {}", sig);
    println!(
        "Message with command_id {:?} has been executed (mocked)",
        command_id
    );
    println!("Payload hash: {:?}", payload_hash);

    Ok(())
}
