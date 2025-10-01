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

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());

    // Gas service program ID
    let gas_program_id = Pubkey::from_str(
        &std::env::var("GAS_PROGRAM_ID")
            .unwrap_or_else(|_| "H9XpBVCnYxr7cHd66nqtD8RSTrKY6JC32XVu2zT2kBmP".to_string()),
    )?;

    // Gateway program ID
    let gateway_program_id = Pubkey::from_str(
        &std::env::var("GATEWAY_PROGRAM_ID")
            .unwrap_or_else(|_| "7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc".to_string()),
    )?;

    let payer_path = std::env::var("PAYER")
        .unwrap_or_else(|_| "/Users/nikos/.config/solana/id.json".to_string());
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    println!("Initializing Programs");
    println!("========================");
    println!("Gas Service: {}", gas_program_id);
    println!("Gateway:     {}", gateway_program_id);
    println!("Payer:       {}", payer.pubkey());
    println!();

    // Derive PDAs
    let (gas_config_pda, _) = Pubkey::find_program_address(&[b"config"], &gas_program_id);
    let (gateway_root_pda, _) = Pubkey::find_program_address(&[b"gateway"], &gateway_program_id);

    println!("PDAs:");
    println!("Gas Config PDA:    {}", gas_config_pda);
    println!("Gateway Root PDA:  {}", gateway_root_pda);
    println!();

    // Initialize Gateway Root PDA
    println!("Initializing Gateway Root PDA...");
    match rpc.get_account(&gateway_root_pda).await {
        Ok(_) => {
            println!("Gateway Root PDA already initialized");
        }
        Err(_) => {
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

            println!("Gateway Root PDA initialized!");
            println!("Transaction: {}", sig);
        }
    }

    // Check Gas Service Config PDA (it doesn't need initialization in this program)
    println!();
    println!("Checking Gas Service Config PDA...");
    match rpc.get_account(&gas_config_pda).await {
        Ok(_) => {
            println!("Gas Config PDA exists");
        }
        Err(_) => {
            println!("Gas Config PDA not initialized (will be created on first use)");
        }
    }

    println!("Initialization Complete!");

    Ok(())
}
