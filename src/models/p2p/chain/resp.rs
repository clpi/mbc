use serde::{Serialize, Deserialize};
use super::Block;


#[derive(Serialize, Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub recv: String,
}