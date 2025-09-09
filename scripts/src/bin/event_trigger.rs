use std::{path::Path, str::FromStr};

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};

fn anchor_sighash(method_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{method_name}"));
    let hash = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let program_id = Pubkey::from_str("DaejccUfXqoAFTiDTxDuMQfQ9oa6crjtR9cT52v1AvGK")?;

    let integer_arg: i64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1);

    // let payer = read_keypair_file(Path::new("/Users/nikos/my-solana-wallet/my-keypair.json"))
    //     .map_err(|e| anyhow!("failed to read keypair: {e}"))?;
    let payer = read_keypair_file(Path::new("/Users/nikos/.config/solana/id.json"))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let mut data: Vec<u8> = Vec::with_capacity(16);
    data.extend_from_slice(&anchor_sighash("emit_received"));
    //data.extend_from_slice(&integer_arg.to_le_bytes());

    let ix = Instruction {
        program_id,
        accounts: vec![],
        data,
    };

    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    let sig = rpc.send_and_confirm_transaction(&tx).await?;
    println!("Event-trigger transaction sent: {}", sig);
    println!("Sent integer: {}", integer_arg);
    Ok(())
}
