use libp2p::request_response::ResponseChannel;

use super::{req::LocalChainReq, resp::ChainResponse};



#[derive(Debug)]
pub enum Event {
    Init,
    OutboundRequest {
        req: String,
        channel: ResponseChannel<LocalChainReq>,
    },
    InboundRequest {
        req: String,
        channel: ResponseChannel<ChainResponse>,
    }
}