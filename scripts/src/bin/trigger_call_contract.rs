use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;

fn anchor_sighash(name: &str) -> [u8; 8] {
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

fn serialize_vec_u8(value: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let program_id = Pubkey::from_str(
        &std::env::var("PROGRAM_ID")
            .unwrap_or_else(|_| "8YsLGnLV2KoyxdksgiAi3gh1WvhMrznA2toKWqyz91bR".to_string()),
    )?;

    let payer_path = std::env::var("PAYER")
        .unwrap_or_else(|_| "/Users/nikos/.config/solana/id.json".to_string());
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);
    let (gateway_root_pda, _gw_bump) = Pubkey::find_program_address(&[b"gateway"], &program_id);
    let (signing_pda, _sig_bump) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &program_id);

    let destination_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let destination_contract_address = std::env::var("DEST_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
    let payload: Vec<u8> = std::env::var("PAYLOAD")
        .ok()
        .map(|s| s.into_bytes())
        .unwrap_or_else(|| vec![1u8, 2, 3, 4, 5]);

    let payload_hash = {
        let digest = Sha256::digest(&payload);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&digest[..32]);
        arr
    };

    // Ensure GatewayConfig exists for call_contract
    if rpc.get_account(&gateway_root_pda).await.is_err() {
        println!("Gateway root PDA not found. Initializing...");
        let ix_init_gateway = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(gateway_root_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: anchor_sighash("init_gateway_root").to_vec(),
        };
        let recent_blockhash = rpc.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[ix_init_gateway], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        let sig = rpc.send_and_confirm_transaction(&tx).await?;
        println!(
            "Initialized gateway_root_pda: {} (tx {})",
            gateway_root_pda, sig
        );
    }

    // Build call_contract instruction
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&anchor_sighash("call_contract"));
    serialize_string(&destination_chain, &mut data);
    serialize_string(&destination_contract_address, &mut data);
    data.extend_from_slice(&payload_hash);
    serialize_vec_u8(&payload, &mut data);

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false), // calling_program
        AccountMeta::new_readonly(signing_pda, false),          // signing_pda
        AccountMeta::new_readonly(gateway_root_pda, false),     // gateway_root_pda
        AccountMeta::new_readonly(event_authority, false),      // event_authority
        AccountMeta::new_readonly(program_id, false),           // program
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    // Send the transaction
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    let sig = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Sent call_contract tx: {}", sig);
    println!("Destination chain: {}", destination_chain);
    println!("Destination address: {}", destination_contract_address);
    println!("Payload hash: {:?}", payload_hash);
    println!("Payload length: {} bytes", payload.len());

    Ok(())
}
