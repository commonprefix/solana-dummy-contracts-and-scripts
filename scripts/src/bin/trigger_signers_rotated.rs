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

fn decode_hex(input: &str) -> Option<Vec<u8>> {
    let s = input.strip_prefix("0x").unwrap_or(input);
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        out.push(byte);
    }
    Some(out)
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

fn anchor_event_struct_discriminator(type_name: &str) -> [u8; 8] {
    // Anchor event struct discriminator = sha256("event:<TypeName>")[..8]
    let mut hasher = Sha256::new();
    hasher.update(format!("event:{type_name}"));
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

    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);

    // Verifier set hash as 32-byte value (hex string like 0x...)
    let verifier_set_hash_hex = std::env::var("VERIFIER_SET_HASH")
        .or_else(|_| std::env::var("SIGNERS_HASH"))
        .unwrap_or_else(|_| {
            "0x1111111111111111111111111111111111111111111111111111111111111111".to_string()
        });
    let verifier_set_hash_raw = decode_hex(&verifier_set_hash_hex)
        .ok_or_else(|| anyhow!("invalid VERIFIER_SET_HASH hex"))?;
    let mut verifier_set_hash = [0u8; 32];
    let copy_len = verifier_set_hash_raw.len().min(32);
    verifier_set_hash[..copy_len].copy_from_slice(&verifier_set_hash_raw[..copy_len]);

    // Epoch as u64, packed little-endian into 32 bytes (U256 LE)
    let epoch_dec: u64 = std::env::var("EPOCH")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);
    let mut epoch_le = [0u8; 32];
    epoch_le[..8].copy_from_slice(&epoch_dec.to_le_bytes());

    let ix = build_signers_rotated_ix(
        &program_id,
        &payer.pubkey(),
        &event_authority,
        &epoch_le,
        &verifier_set_hash,
    )?;

    let sig = send_ix(&rpc, &payer, &[ix]).await?;
    println!("Sent signers_rotated tx: {}", sig);

    let rotated_disc = anchor_event_struct_discriminator("VerifierSetRotatedEvent");
    println!(
        "VerifierSetRotatedEvent discriminator: {:#04x?}",
        rotated_disc
    );

    Ok(())
}

fn build_signers_rotated_ix(
    program_id: &Pubkey,
    payer: &Pubkey,
    event_authority: &Pubkey,
    epoch_le: &[u8; 32],
    verifier_set_hash: &[u8; 32],
) -> Result<Instruction> {
    let accounts = vec![
        AccountMeta::new(*payer, true), // payer: Signer, mut
        AccountMeta::new_readonly(*event_authority, false), // event_authority
        AccountMeta::new_readonly(*program_id, false), // program
    ];

    let disc = anchor_method_discriminator("signers_rotated");
    let mut data = Vec::with_capacity(8 + 32 + 32);
    data.extend_from_slice(&disc);
    data.extend_from_slice(epoch_le);
    data.extend_from_slice(verifier_set_hash);

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
