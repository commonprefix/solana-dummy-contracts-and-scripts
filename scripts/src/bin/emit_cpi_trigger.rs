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
    let hash = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

fn serialize_string(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = "http://127.0.0.1:8899".to_string();

    // Gas service program ID
    let program_id = Pubkey::from_str(
        &std::env::var("GAS_PROGRAM_ID")
            .unwrap_or_else(|_| "H9XpBVCnYxr7cHd66nqtD8RSTrKY6JC32XVu2zT2kBmP".to_string()),
    )?;

    let payer_path = "/Users/nikos/.config/solana/id.json".to_string();
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);

    let destination_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let destination_address = "0x7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fdd".to_string();
    let payload_hash = {
        let mut arr = [0u8; 32];
        arr[..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        arr
    };
    let refund_address = payer.pubkey();
    let gas_fee_amount: u64 = 1_000;

    let mut data: Vec<u8> = Vec::with_capacity(8 + 128);
    data.extend_from_slice(&anchor_sighash("pay_native_for_contract_call"));
    serialize_string(&destination_chain, &mut data);
    serialize_string(&destination_address, &mut data);
    data.extend_from_slice(&payload_hash);
    data.extend_from_slice(refund_address.as_ref());
    data.extend_from_slice(&gas_fee_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true), // payer: Signer, mut
        AccountMeta::new_readonly(config_pda, false), // config_pda: UncheckedAccount
        AccountMeta::new_readonly(system_program::id(), false), // system_program
        AccountMeta::new_readonly(event_authority, false), // PDA; not a signer in outer tx
        AccountMeta::new_readonly(program_id, false), // program: the program itself
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    println!("tx: {:?}", tx);

    let sig = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Sent pay_native_for_contract_call tx: {}", sig);
    Ok(())
}
