use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

#[tokio::main]
async fn main() {
    
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    
    let from_keypair = Keypair::new();
    let to_pubkey = Pubkey::from_str("EfNMxEv6RpJLmntFYCSYmy6nBG1NW3SJ2tMzr1cw6cL7").unwrap();

    
    let balance = client.get_balance(&from_keypair.pubkey()).unwrap();
    println!("Balance: {}", balance);

    
    let latest_blockhash = client.get_latest_blockhash().unwrap();
    let transfer_ix = system_instruction::transfer(
        &from_keypair.pubkey(),
        &to_pubkey,
        1_000_000, // 0.001 SOL
    );

    let transaction = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&from_keypair.pubkey()),
        &[&from_keypair],
        latest_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&transaction).unwrap();
    println!("Transaction signature: {}", signature);
}