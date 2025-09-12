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

const CONFIG_SEED: &[u8] = b"gateway"; // for gateway_root_pda
const SIG_SEED: &[u8] = b"gtw-sig-verif";

fn anchor_method_discriminator(name: &str) -> [u8; 8] {
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
    let program_id = Pubkey::from_str(
        &std::env::var("PROGRAM_ID")
            .unwrap_or_else(|_| "7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc".to_string()),
    )?;

    let payer_path = std::env::var("PAYER")
        .unwrap_or_else(|_| "/Users/nikos/.config/solana/id.json".to_string());
    let payer = read_keypair_file(Path::new(&payer_path))
        .map_err(|e| anyhow!("failed to read keypair: {e}"))?;

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let (gateway_root_pda, _gw_bump) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &program_id);

    // Ensure gateway_root exists
    if rpc.get_account(&gateway_root_pda).await.is_err() {
        let ix_init_gateway = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(gateway_root_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: anchor_method_discriminator("init_gateway_root").to_vec(),
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

    // Build dummy MerkleisedMessage with timestamp for uniqueness
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let cc_chain = std::env::var("SRC_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let cc_id = std::env::var("SRC_ID").unwrap_or_else(|_| format!("0x{:x}", timestamp));
    let src_address = std::env::var("SRC_ADDR").unwrap_or_else(|_| "0xdead".to_string());
    let dst_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "solana".to_string());
    let dst_address = std::env::var("DEST_ADDR").unwrap_or_else(|_| payer.pubkey().to_string());

    let mut payload_merkle_root = [0u8; 32];
    let root_input = format!("dummy-root-{}", timestamp);
    payload_merkle_root.copy_from_slice(&Sha256::digest(root_input.as_bytes())[..32]);

    // Serialize MerkleisedMessage (borsh/anchor layout)
    // Message { cc_id { chain, id }, source_address, destination_chain, destination_address, payload_hash }
    let mut message = Vec::new();
    // cc_id.chain
    put_string(&cc_chain, &mut message);
    // cc_id.id
    put_string(&cc_id, &mut message);
    // source_address
    put_string(&src_address, &mut message);
    // destination_chain
    put_string(&dst_chain, &mut message);
    // destination_address
    put_string(&dst_address, &mut message);
    // payload_hash (dummy from text)
    let mut payload_hash = [0u8; 32];
    payload_hash.copy_from_slice(&Sha256::digest(b"payload")[..32]);
    message.extend_from_slice(&payload_hash);

    // Compute command_id for incoming_message PDA seeds
    let command_id = keccak::hashv(&[cc_chain.as_bytes(), b"-", cc_id.as_bytes()]).0;

    // MessageLeaf { message, position: u16, set_size: u16, domain_separator: [u8;32], signing_verifier_set: [u8;32] }
    let mut leaf = Vec::new();
    leaf.extend_from_slice(&message); // nested struct without length prefix
    leaf.extend_from_slice(&0u16.to_le_bytes()); // position
    leaf.extend_from_slice(&1u16.to_le_bytes()); // set_size
    leaf.extend_from_slice(&[0u8; 32]); // domain_separator
    leaf.extend_from_slice(&[0u8; 32]); // signing_verifier_set

    // MerkleisedMessage { leaf, proof: Vec<u8> }
    let mut merkle_msg = Vec::new();
    merkle_msg.extend_from_slice(&leaf);
    merkle_msg.extend_from_slice(&0u32.to_le_bytes()); // empty proof vec

    // Build approve_message data: discriminator + MerkleisedMessage + payload_merkle_root
    let mut data = Vec::with_capacity(8 + merkle_msg.len() + 32);
    data.extend_from_slice(&anchor_method_discriminator("approve_message"));
    data.extend_from_slice(&merkle_msg);
    data.extend_from_slice(&payload_merkle_root);

    // Accounts for ApproveMessage
    let (verification_session_account, _vs_bump) =
        Pubkey::find_program_address(&[SIG_SEED, payload_merkle_root.as_ref()], &program_id);
    let (incoming_message_pda, _in_bump) =
        Pubkey::find_program_address(&[b"incoming message", &command_id], &program_id);

    // Ensure verification session exists
    if rpc
        .get_account(&verification_session_account)
        .await
        .is_err()
    {
        let mut init_vs_data = anchor_method_discriminator("init_verification_session").to_vec();
        init_vs_data.extend_from_slice(&payload_merkle_root);
        let ix_init_vs = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(verification_session_account, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: init_vs_data,
        };
        let recent_blockhash = rpc.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[ix_init_vs], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        let sig = rpc.send_and_confirm_transaction(&tx).await?;
        println!(
            "Initialized verification_session_account: {} (tx {})",
            verification_session_account, sig
        );
    }

    let accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(payer.pubkey(), true), // funder
        AccountMeta::new(verification_session_account, false),
        AccountMeta::new(incoming_message_pda, false),
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

    let sig = send_ix(&rpc, &payer, &[ix]).await?;
    println!("Sent approve_message tx: {}", sig);

    Ok(())
}

fn put_string(s: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
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
