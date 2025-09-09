use std::path::Path;

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    signature::{read_keypair_file, Signer},
    signer::keypair::Keypair,
    system_instruction, system_program,
    transaction::Transaction,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a connection to cluster
    let connection = RpcClient::new_with_commitment(
        "https://api.devnet.solana.com".to_string(),
        CommitmentConfig::confirmed(),
    );

    let sender = read_keypair_file(Path::new("/Users/nikos/my-solana-wallet/my-keypair.json"))
        .map_err(|e| anyhow::anyhow!("Error reading keypair file: {}", e))?;
    let recipient = Keypair::new();

    // Fund sender with airdrop
    // let airdrop_signature = connection
    //     .request_airdrop(&sender.pubkey(), LAMPORTS_PER_SOL)
    //     .await?;
    // loop {
    //     let confirmed = connection.confirm_transaction(&airdrop_signature).await?;
    //     if confirmed {
    //         break;
    //     }
    // }

    // Check balance before transfer
    let pre_balance1 = connection.get_balance(&sender.pubkey()).await?;
    let pre_balance2 = connection.get_balance(&recipient.pubkey()).await?;

    println!("Sender prebalance: {}", pre_balance1);
    println!("Recipient prebalance: {}", pre_balance2);

    // let transfer_instruction =
    //     system_instruction::transfer(&sender.pubkey(), &recipient.pubkey(), transfer_amount);

    // equivilently:

    let transfer_instruction_index: u32 = 2;

    let transfer_amount = LAMPORTS_PER_SOL / 100; // 0.01 SOL

    let mut instruction_data = Vec::with_capacity(12);
    instruction_data.extend_from_slice(&transfer_instruction_index.to_le_bytes());
    instruction_data.extend_from_slice(&transfer_amount.to_le_bytes());

    let transfer_instruction = Instruction {
        program_id: system_program::id(),
        accounts: vec![
            AccountMeta::new(sender.pubkey(), true), // from account, is signer and is writable
            AccountMeta::new(recipient.pubkey(), false), // to account, is not signer but is writable
        ],
        data: instruction_data,
    };

    let mut transaction =
        Transaction::new_with_payer(&[transfer_instruction], Some(&sender.pubkey()));
    let blockhash = connection.get_latest_blockhash().await?;
    transaction.sign(&[&sender], blockhash);

    let transaction_signature = connection
        .send_and_confirm_transaction(&transaction)
        .await?;

    let post_balance1 = connection.get_balance(&sender.pubkey()).await?;
    let post_balance2 = connection.get_balance(&recipient.pubkey()).await?;

    println!(
        "Sender prebalance: {}",
        pre_balance1 as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "Recipient prebalance: {}",
        pre_balance2 as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "Sender postbalance: {}",
        post_balance1 as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "Recipient postbalance: {}",
        post_balance2 as f64 / LAMPORTS_PER_SOL as f64
    );
    println!("Transaction Signature: {}", transaction_signature);

    Ok(())
}
