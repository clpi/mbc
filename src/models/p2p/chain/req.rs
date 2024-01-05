
use std::collections::HashSet;

use libp2p::{Multiaddr, PeerId, request_response::ResponseChannel};
use serde::{Serialize, Deserialize};

use crate::models::{SendResult, Sender};

use super::resp::ChainResponse;

#[derive(Serialize, Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LocalChainReq {
    pub from_peer_id: String,
}

#[derive(Debug)]
pub enum Command {
    StartListening {
        addr: libp2p::Multiaddr,
        send: SendResult<()>,
    },
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        send: SendResult<()>,
    },
    StartProviding {
        file_name: String,
        send: Sender<()>,
    },
    GetProviders {
        file_name: String,
        send: Sender<HashSet<PeerId>>,
    },
    RequestFile {
        file_name: String,
        peer_id: PeerId,
        send: SendResult<Vec<u8>>,
    },
    RespondFile {
        file: Vec<u8>,
        channel: ResponseChannel<ChainResponse>,
    }
}