use std::str::FromStr;
use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status_client_types::UiTransactionEncoding;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commitment = Some(CommitmentConfig::confirmed());
    let limit = 2;
    // let before = Some(Signature::from_str(
    //     "5Pg6SHHKCBEz4yHtnsiK7EtTvnPk31WQ9Adh48XhwcDv7ghwLY4ADvTneq3bw64osqZwjwehVRBrKwDG2XNzrvFB",
    // )?);
    // let until = Some(Signature::from_str(
    //     "9oZ5CpmvpMo3szrFPNS9hnzsxQ6D4hVYJbKHsm4VBcmFf2WPsGFJYq5D42K4SRKfBMx4kXNtUycxFtWW6Tu1M9a",
    // )?);
    let before = None;
    let until = None;
    let vec_sigs: Vec<Signature> = vec![];

    let rpc_url = "http://localhost:8899".to_string();
    let program_id = Pubkey::from_str("7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc")?;
    let config = GetConfirmedSignaturesForAddress2Config {
        commitment,
        limit: Some(limit),
        before,
        until,
    };

    let client = Arc::new(RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ));
    match client
        .get_signatures_for_address_with_config(&program_id, config)
        .await
    {
        Ok(sigs) => {
            let mut handles = Vec::new();
            for sig in sigs {
                let client = Arc::clone(&client);
                let handle = tokio::spawn(async move {
                    println!("Signature: {}", sig.signature);
                    let tx = client
                        .get_transaction_with_config(
                            &Signature::from_str(&sig.signature).unwrap(),
                            RpcTransactionConfig {
                                encoding: Some(UiTransactionEncoding::Json),
                                commitment: Some(CommitmentConfig::confirmed()),
                                max_supported_transaction_version: None,
                            },
                        )
                        .await;
                    if let Ok(tx) = tx {
                        println!("Transaction got");
                        //println!("Transaction: {:?}", tx);
                    } else {
                        println!("Error: {:?}", tx.err());
                    }
                });
                handles.push(handle);
            }
            for handle in handles {
                let _ = handle.await;
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    Ok(())
}
