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

fn serialize_vec_u8(value: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

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

    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &gas_program_id);
    let (gas_event_authority, _ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &gas_program_id);
    let (gateway_root_pda, _gw_bump) =
        Pubkey::find_program_address(&[b"gateway"], &gateway_program_id);
    let (gateway_event_authority, _gw_ea_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program_id);
    let (signing_pda, _sig_bump) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &gateway_program_id);

    let destination_chain = std::env::var("DEST_CHAIN").unwrap_or_else(|_| "ethereum".to_string());
    let destination_address = std::env::var("DEST_ADDRESS")
        .unwrap_or_else(|_| "0x7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fdd".to_string());
    let payload: Vec<u8> = std::env::var("PAYLOAD_HEX")
        .ok()
        .and_then(|hex| decode_hex(&hex))
        .unwrap_or_else(|| vec![1u8, 2, 3, 4, 5]);

    let payload_hash = {
        if let Ok(hex) = std::env::var("PAYLOAD_HASH_HEX") {
            let raw = decode_hex(&hex).unwrap_or_default();
            let mut arr = [0u8; 32];
            arr[..raw.len().min(32)].copy_from_slice(&raw[..raw.len().min(32)]);
            arr
        } else {
            let digest = Sha256::digest(&payload);
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&digest[..32]);
            arr
        }
    };

    let refund_address = payer.pubkey();
    let gas_fee_amount: u64 = std::env::var("GAS_FEE_AMOUNT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_000);

    let mut data_pay_native: Vec<u8> = Vec::with_capacity(8 + 128);
    data_pay_native.extend_from_slice(&anchor_sighash("pay_native_for_contract_call"));
    serialize_string(&destination_chain, &mut data_pay_native);
    serialize_string(&destination_address, &mut data_pay_native);
    data_pay_native.extend_from_slice(&payload_hash);
    data_pay_native.extend_from_slice(refund_address.as_ref());
    data_pay_native.extend_from_slice(&gas_fee_amount.to_le_bytes());

    let accounts_pay_native = vec![
        AccountMeta::new(payer.pubkey(), true), // payer: Signer, mut
        AccountMeta::new_readonly(config_pda, false), // config_pda: UncheckedAccount
        AccountMeta::new_readonly(system_program::id(), false), // system_program
        // Event CPI injected accounts (must be last two): event_authority and program
        AccountMeta::new_readonly(gas_event_authority, false),
        AccountMeta::new_readonly(gas_program_id, false),
    ];

    let ix_pay_native = Instruction {
        program_id: gas_program_id,
        accounts: accounts_pay_native,
        data: data_pay_native,
    };

    // Ensure GatewayConfig exists for call_contract
    if rpc.get_account(&gateway_root_pda).await.is_err() {
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
    }

    let mut data_call: Vec<u8> = Vec::with_capacity(8 + 256);
    data_call.extend_from_slice(&anchor_sighash("call_contract"));
    serialize_string(&destination_chain, &mut data_call);
    serialize_string(&destination_address, &mut data_call); // destination_contract_address
    data_call.extend_from_slice(&payload_hash);
    serialize_vec_u8(&payload, &mut data_call);

    let accounts_call = vec![
        // CallContract accounts
        AccountMeta::new_readonly(system_program::id(), false), // calling_program (any executable prog)
        AccountMeta::new_readonly(signing_pda, false),          // signing_pda (dummy PDA)
        AccountMeta::new_readonly(gateway_root_pda, false),     // GatewayConfig
        // Event CPI injected accounts (must be last two)
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new_readonly(gateway_program_id, false),
    ];

    let ix_call = Instruction {
        program_id: gateway_program_id,
        accounts: accounts_call,
        data: data_call,
    };

    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix_pay_native, ix_call], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    let sig = rpc.send_and_confirm_transaction(&tx).await?;
    println!(
        "Sent pay_native_for_contract_call + call_contract tx: {}",
        sig
    );

    Ok(())
}
