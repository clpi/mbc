use mbc::models::{PEER_ID, KEYS};

pub const BOOTNODES: [&str; 4] = [
    "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let (resp_send, mut resp_recv) = mpsc::unbounded_channel();
    // let (init_send, mut init_recv) = mpsc::unbounded_channel();
    tracing::info!("Peer id: {}", PEER_ID.clone());
    tracing::info!("Auth keys: {:?}", KEYS.clone());
    mbc::init().await
}


