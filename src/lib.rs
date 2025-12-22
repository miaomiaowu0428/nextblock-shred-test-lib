pub mod protos {
    pub mod nextblock_stream;
}

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dotenvy::dotenv;
use rand::Rng;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::VersionedTransaction,
};

use anyhow::{Context, Result, bail};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};

use crate::protos::nextblock_stream::{
    NextStreamSubscription, next_stream_service_client::NextStreamServiceClient,
};

use sol_tx_send::constants::REGION;
use sol_tx_send::platform_clients::Region;

// #[path = "protos/next_block_stream.rs"]
// pub mod next_block_stream;

/*
the auth message needs to be built clients-side.
it is a pipe-separated string made of:
1. domain that's being connected to (e.g. fra.stream.nextblock.io)
2. the publickey that sent the fee to strmuYvHKeA1qvHqooUpwUk2BFwaAmMbK9WXY9mh2GJ
3. a random nonce
4. the current unix timestamp

it is then signed by the supplied publickey.
*/

fn build_auth_message(domain: &str, pubkey: &Pubkey) -> String {
    let nonce: u64 = rand::thread_rng().r#gen();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}|{}|{}|{}", domain, pubkey.to_string(), nonce, ts)
}

async fn make_insecure_channel(domain: &str) -> Result<Channel> {
    let ep = Endpoint::from_shared(format!("http://{}", domain))?
        .http2_keep_alive_interval(Duration::from_secs(5))
        .keep_alive_while_idle(false);
    Ok(ep.connect().await?)
}

// Frankfurt: fra.stream.nextblock.io:22221
// Amsterdam: amsterdam.stream.nextblock.io:22221
// London: london.stream.nextblock.io:22221
// Singapore: singapore.stream.nextblock.io:22221
// Tokyo: tokyo.stream.nextblock.io:22221
// New York: ny.stream.nextblock.io:22221
// Salt Lake City: slc.stream.nextblock.io:22221
// Dublin: dublin.stream.nextblock.io:22221
// Vilnius: vilnius.stream.nextblock.io:22221

fn get_domain() -> Result<String> {
    match *REGION {
        Region::NewYork => Ok("nyc.stream.nextblock.io:22221".to_string()),
        Region::Frankfurt => Ok("fra.stream.nextblock.io:22221".to_string()),
        Region::Amsterdam => Ok("amsterdam.stream.nextblock.io:22221".to_string()),
        Region::London => Ok("london.stream.nextblock.io:22221".to_string()),
        Region::Singapore => Ok("singapore.stream.nextblock.io:22221".to_string()),
        Region::Tokyo => Ok("tokyo.stream.nextblock.io:22221".to_string()),
        Region::SaltLakeCity => Ok("slc.stream.nextblock.io:22221".to_string()),
        // Region::Dublin => Ok("dublin.stream.nextblock.io:22221".to_string()),
        // Region::Vilnius => Ok("vilnius.stream.nextblock.io:22221".to_string()),
        _ => Ok("fra.stream.nextblock.io:22221".to_string()),
    }
}

use std::env;
use std::sync::LazyLock;
/// 早期卖出黑名单文件路径
pub static NEXT_BLOCK_SHRED_PRIVATE_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("NEXT_BLOCK_SHRED_PRIVATE_KEY").unwrap_or_else(|_| "".to_string()));

pub async fn nextblock_shred_monitor() -> Result<()> {
    dotenv().ok();
    let domain = get_domain()?;
    let private_key_b58 = &*NEXT_BLOCK_SHRED_PRIVATE_KEY;
    if private_key_b58.is_empty() {
        panic!("Set `NEXT_BLOCK_SHRED_PRIVATE_KEY` to your base58-encoded Solana private key.");
    }

    let authentication_keypair = Keypair::from_base58_string(private_key_b58);
    let authentication_pubkey = authentication_keypair.pubkey();
    println!("pubkey: {authentication_pubkey}");

    let authentication_message = build_auth_message(&domain, &authentication_pubkey);
    let authentication_signature =
        authentication_keypair.sign_message(authentication_message.as_bytes());

    let channel = make_insecure_channel(&domain).await?;
    let mut client = NextStreamServiceClient::new(channel);

    let req = NextStreamSubscription {
        authentication_publickey: authentication_pubkey.to_string(),
        authentication_message: authentication_message,
        authentication_signature: authentication_signature.to_string(),
        accounts: vec![],
    };

    let mut stream = client
        .subscribe_next_stream(Request::new(req))
        .await?
        .into_inner();

    while let Some(msg) = stream.message().await.context("recv")? {
        if let Some(packet) = msg.packet {
            let tx: VersionedTransaction = bincode::deserialize(&packet.transaction)?;
            let first_sig = tx
                .signatures
                .get(0)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<no signatures>".to_string());
            println!("got new sig {} on slot {}", first_sig, packet.slot);
        }
    }
    Ok(())
}
