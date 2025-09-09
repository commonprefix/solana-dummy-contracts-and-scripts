use std::backtrace;
use std::collections::HashSet;
use std::str::FromStr;
use std::time::Duration;

use futures::StreamExt;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status_client_types::{UiInstruction, UiMessage, UiTransactionEncoding};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc_url = "http://localhost:8899".to_string();

    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let pub_sub_client = PubsubClient::new("ws://localhost:8900").await?;

    let (mut sub, _unsub) = pub_sub_client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![
                "7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc".to_string(),
            ]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )
        .await?;

    println!("Listening for events...");

    while let Some(msg) = sub.next().await {
        println!("msg: {:?}", msg);
        let tx = client
            .get_transaction_with_config(
                &Signature::from_str(&msg.value.signature).unwrap(),
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: None,
                },
            )
            .await
            .unwrap();

        println!("--------------------------------");

        println!("tx: {:?}", tx);

        println!("--------------------------------");

        if let Some(meta) = &tx.transaction.meta {
            let inner_opt: Option<
                Vec<solana_transaction_status_client_types::UiInnerInstructions>,
            > = (&meta.inner_instructions).clone().into();
            if let Some(inner) = inner_opt {
                for group in inner.into_iter() {
                    for inst in group.instructions.into_iter() {
                        if let solana_transaction_status_client_types::UiInstruction::Compiled(ci) =
                            inst
                        {
                            if let solana_transaction_status_client_types::EncodedTransaction::Json(
                                ui_tx,
                            ) = &tx.transaction.transaction
                            {
                                if let UiMessage::Raw(raw_msg) = &ui_tx.message {
                                    let keys = &raw_msg.account_keys;
                                    if (ci.program_id_index as usize) < keys.len()
                                        && keys[ci.program_id_index as usize]
                                            == "7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc"
                                    {
                                        let bytes = match bs58::decode(&ci.data).into_vec() {
                                            Ok(v) => v,
                                            Err(_) => continue,
                                        };
                                        if bytes.len() < 16 {
                                            continue;
                                        }

                                        let mut i = 16usize;
                                        let n = bytes.len();

                                        fn take_slice<'a>(bytes: &'a [u8], i: &mut usize, len: usize) -> Option<&'a [u8]> {
                                            if *i + len > bytes.len() { None } else {
                                                let out = &bytes[*i..*i + len];
                                                *i += len;
                                                Some(out)
                                            }
                                        }

                                        fn read_pubkey(bytes: &[u8], i: &mut usize) -> Option<Pubkey> {
                                            let s = take_slice(bytes, i, 32)?;
                                            let mut arr = [0u8; 32];
                                            arr.copy_from_slice(s);
                                            Some(Pubkey::new_from_array(arr))
                                        }

                                        fn read_u32(bytes: &[u8], i: &mut usize) -> Option<u32> {
                                            let s = take_slice(bytes, i, 4)?;
                                            let mut lenb = [0u8; 4];
                                            lenb.copy_from_slice(s);
                                            Some(u32::from_le_bytes(lenb))
                                        }

                                        fn read_string(bytes: &[u8], i: &mut usize) -> Option<String> {
                                            let len = read_u32(bytes, i)? as usize;
                                            let s = take_slice(bytes, i, len)?;
                                            Some(std::str::from_utf8(s).ok()?.to_string())
                                        }

                                        fn read_vec_u8(bytes: &[u8], i: &mut usize) -> Option<Vec<u8>> {
                                            let len = read_u32(bytes, i)? as usize;
                                            let s = take_slice(bytes, i, len)?;
                                            Some(s.to_vec())
                                        }

                                        let config_pda = match read_pubkey(&bytes, &mut i) { Some(v) => v, None => continue };
                                        let destination_chain = match read_string(&bytes, &mut i) { Some(v) => v, None => continue };
                                        let destination_address = match read_string(&bytes, &mut i) { Some(v) => v, None => continue };
                                        let payload_hash = match take_slice(&bytes, &mut i, 32) {
                                            Some(s) => {
                                                let mut arr = [0u8; 32];
                                                arr.copy_from_slice(s);
                                                arr
                                            }
                                            None => continue,
                                        };
                                        let refund_address = match read_pubkey(&bytes, &mut i) { Some(v) => v, None => continue };
                                        let params = match read_vec_u8(&bytes, &mut i) { Some(v) => v, None => continue };
                                        let gas_fee_amount = match take_slice(&bytes, &mut i, 8) {
                                            Some(s) => {
                                                let mut gasb = [0u8; 8];
                                                gasb.copy_from_slice(s);
                                                u64::from_le_bytes(gasb)
                                            }
                                            None => continue,
                                        };

                                        println!("Decoded Event:");
                                        println!("  config_pda: {}", config_pda);
                                        println!("  destination_chain: {}", destination_chain);
                                        println!("  destination_address: {}", destination_address);
                                        println!("  payload_hash[0..4]: {:?}", &payload_hash[..4]);
                                        println!("  refund_address: {}", refund_address);
                                        println!("  params: {:?}", params);
                                        println!("  gas_fee_amount: {}", gas_fee_amount);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
