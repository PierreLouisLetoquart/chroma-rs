use chroma_rs::client::{ChromaClient, ChromaClientParams};
use std::{error::Error, result::Result};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = ChromaClient::new(ChromaClientParams {
        host: "localhost".to_string(),
        port: "8000".to_string(),
        ssl: false,
    });

    let hb = client.heartbeat().await?;

    println!("{hb}");

    Ok(())
}