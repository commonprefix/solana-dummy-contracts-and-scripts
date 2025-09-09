use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use reqwest;
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;

#[derive(Deserialize)]
struct JsonRpcItem {
    id: usize,
    #[serde(default)]
    result: serde_json::Value,
    #[serde(default)]
    error: serde_json::Value,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc_url = "http://localhost:8899".to_string(); // use some RPC that supports batching
    let program_id = Pubkey::from_str("DaejccUfXqoAFTiDTxDuMQfQ9oa6crjtR9cT52v1AvGK")?;

    let client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());
    let http = reqwest::Client::new();

    let mut seen: HashSet<String> = HashSet::new();

    let sigs = client
        .get_signatures_for_address_with_config(
            &program_id,
            GetConfirmedSignaturesForAddress2Config {
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
        )
        .await
        .context("get_signatures_for_address failed")?;

    let new_sigs: Vec<_> = sigs
        .iter()
        .rev()
        .filter_map(|x| {
            let sig = x.signature.clone();
            if seen.contains(&sig) {
                None
            } else {
                Some((sig, x.slot))
            }
        })
        .collect();

    if !new_sigs.is_empty() {
        let mut id_to_sig: HashMap<usize, (String, u64)> = HashMap::new();
        let cfg = json!({
          "commitment": "confirmed",
          "maxSupportedTransactionVersion": 0,
          "encoding": "json"
        });
        let batch: Vec<serde_json::Value> = new_sigs
            .iter()
            .enumerate()
            .map(|(i, (sig, _slot))| {
                let id = i + 1;
                id_to_sig.insert(id, (sig.clone(), *_slot));
                json!({
                  "jsonrpc": "2.0",
                  "id": id,
                  "method": "getTransaction",
                  "params": [ sig, cfg ]
                })
            })
            .collect();

        let resp = http
            .post(&rpc_url)
            .json(&batch)
            .send()
            .await
            .context("getTransaction batch request failed")?;

        let items: Vec<JsonRpcItem> = resp
            .json()
            .await
            .context("failed to parse JSON-RPC batch response")?;

        println!("{}", items.len());

        for item in items {
            let (sig, slot) = id_to_sig
                .get(&item.id)
                .cloned()
                .unwrap_or_else(|| ("<unknown>".to_string(), 0));

            if !item.error.is_null() {
                eprintln!("error for {}: {}", sig, item.error);
                continue;
            }

            let meta = item.result.get("meta");
            let logs_len = meta
                .and_then(|m| m.get("logMessages"))
                .and_then(|lm| lm.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
        }
    }
    Ok(())
}
