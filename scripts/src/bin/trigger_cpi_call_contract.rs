use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    system_program,
    transaction::Transaction,
};
use std::path::Path;
use std::str::FromStr;

fn anchor_sighash(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&Sha256::digest(preimage.as_bytes())[..8]);
    sighash
}

fn serialize_string(s: &str, buf: &mut Vec<u8>) {
    let len = s.len() as u32;
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

fn serialize_vec_u8(v: &[u8], buf: &mut Vec<u8>) {
    let len = v.len() as u32;
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(v);
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());

    // Gas service program ID
    let gas_program_id = Pubkey::from_str(
        &std::env::var("GAS_PROGRAM_ID")
            .unwrap_or_else(|_| "CJ9f8WFdm3q38pmg426xQf7uum7RqvrmS9R58usHwNX7".to_string()),
    )?;

    // Gateway program ID (program_tester)
    let gateway_program_id = Pubkey::from_str(
        &std::env::var("GATEWAY_PROGRAM_ID")
            .unwrap_or_else(|_| "8YsLGnLV2KoyxdksgiAi3gh1WvhMrznA2toKWqyz91bR".to_string()),
    )?;

    let payer_path = std::env::var("PAYER")
        .unwrap_or_else(|_| "/Users/nikos/.config/solana/id.json".to_string());
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Derive necessary PDAs
    let (gateway_root_pda, _gw_bump) =
        Pubkey::find_program_address(&[b"gateway"], &gateway_program_id);
    let (signing_pda, _sig_bump) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &gateway_program_id);
    let (gateway_event_authority, _gw_ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program_id);

    // Set up call parameters
    let destination_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let destination_contract_address = std::env::var("DEST_ADDRESS")
        .unwrap_or_else(|_| "0x1234567890123456789012345678901234567890".to_string());
    let payload: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let payload_hash = {
        let digest = Sha256::digest(&payload);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&digest[..32]);
        arr
    };

    println!("Gas Service Program ID: {}", gas_program_id);
    println!(
        "Gateway Program ID (program_tester): {}",
        gateway_program_id
    );
    println!("Gateway Root PDA: {}", gateway_root_pda);
    println!("Signing PDA: {}", signing_pda);
    println!("Destination Chain: {}", destination_chain);
    println!("Destination Address: {}", destination_contract_address);

    // Ensure GatewayConfig exists for call_contract
    if rpc.get_account(&gateway_root_pda).await.is_err() {
        println!("Gateway root PDA not found. Initializing...");
        let ix_init_gateway = Instruction {
            program_id: gateway_program_id,
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
    } else {
        println!("Gateway root PDA already exists");
    }

    // Build cpi_call_contract instruction
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&anchor_sighash("cpi_call_contract"));
    serialize_string(&destination_chain, &mut data);
    serialize_string(&destination_contract_address, &mut data);
    data.extend_from_slice(&payload_hash);
    serialize_vec_u8(&payload, &mut data);

    // Accounts for CpiCallContract
    // Following the order in the CpiCallContract struct
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),               // payer
        AccountMeta::new_readonly(gateway_program_id, false), // program_tester_program
        AccountMeta::new_readonly(gas_program_id, false),     // gas_service_program
        AccountMeta::new_readonly(signing_pda, false),        // signing_pda
        AccountMeta::new_readonly(gateway_root_pda, false),   // gateway_root_pda
        AccountMeta::new_readonly(gateway_event_authority, false), // event_authority
        AccountMeta::new_readonly(system_program::id(), false), // system_program
    ];

    let ix = Instruction {
        program_id: gas_program_id,
        accounts,
        data,
    };

    // Send the transaction
    println!("\nSending CPI call_contract transaction...");
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    let sig = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Transaction signature: {}", sig);
    println!("This demonstrates:");
    println!("1. gas_service's cpi_call_contract function makes a CPI call to program_tester");
    println!("2. program_tester's call_contract emits an event using emit_cpi!");
    println!("3. The event is emitted as a self-CPI within the CPI context");

    Ok(())
}
