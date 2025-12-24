pub mod protos {
    pub mod tempo;
}
pub static TEMPO_SHRED_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("TEMPO_SHRED_KEY").unwrap_or_else(|_| "".to_string()));

use std::{env, sync::LazyLock};

use crate::protos::tempo::{
    StartStreamV2, Transaction, transaction_stream_client::TransactionStreamClient,
};
use log::{error, info};
use sol_tx_send::{constants::REGION, platform_clients::Region};
use solana_sdk::{pubkey, transaction::VersionedTransaction};

fn get_domain() -> String {
    match *REGION {
        Region::NewYork => "https://ewr1.beta.tempo.temporal.xyz:50051".to_string(),
        Region::Frankfurt => "https://fra1.beta.tempo.temporal.xyz:50051".to_string(),
        Region::Amsterdam => "https://ams1.beta.tempo.temporal.xyz:50051".to_string(),
        Region::London => "https://lon1.beta.tempo.temporal.xyz:50051".to_string(),
        _ => "https://fra1.beta.tempo.temporal.xyz:50051".to_string(),
    }
}

pub async fn tempo_shred_monitor() {
    // Connect to endpoint
    let endpoint = get_domain();
    println!("Region: {:?}",*REGION);
    println!("endpoint: {}",endpoint);

    let mut client = TransactionStreamClient::connect(endpoint)
        .await
        .expect("fail to connect to tempo shred server");

    // Send request to start stream
    let auth_token = TEMPO_SHRED_KEY.clone();
    let start_stream_request = StartStreamV2 {
        auth_token,
        static_account_filter: vec![
            pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
                .to_bytes()
                .to_vec(),
            pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
                .to_bytes()
                .to_vec(),
        ],
    };
    let mut stream = client
        .open_transaction_stream_v2(start_stream_request)
        .await
        .expect("fail to build tempo shred stream");

    // Process transactions from stream
    loop {
        match stream.get_mut().message().await {
            // Next message successfully received
            Ok(Some(Transaction {
                // Slot for the shred this transaction was found in
                slot,
                // The transaction's index within the block in this slot
                index,
                // The transaction bytes
                payload,
            })) => {
                let Ok(tx) = bincode::deserialize::<VersionedTransaction>(&payload) else {
                    continue;
                };
                let Some(sig) = tx.signatures.get(0) else {
                    continue;
                };
                println!("found tx: {sig} in slot: {slot}")
            }

            // Stream was closed. Should not happen unless server goes down.
            // May want some stream reconnect logic
            Ok(None) => {
                info!("TransactionStream closed!");
                return;
            }

            // Invalid Message. Also should not happen. Maybe yell at us
            Err(e) => {
                error!("invalid stream message! error is {e:?}");
                return;
            }
        }
    }
}
