#![allow()]
use libp2p::swarm::NetworkBehaviour;
use ring::digest::{Context, Digest, SHA256};
use chrono::Utc;
use log::info;
use serde::{Serialize, Deserialize};

use std::{
    io::{BufReader, Read, Write},
    fmt::{self, Debug, Formatter},
    string::ToString,
    hash::{Hash, Hasher},
    convert::{TryFrom}, str::Bytes,
};
const DIFFICULTY_PREFIX: &str = "00";

// #[derive(NetworkBehaviour)]
pub struct Chain {
    pub blocks: Vec<Block>,
    pub difficulty: u32,
}

#[derive(Hash, Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {

}
impl ToString for Transaction {
    fn to_string(&self) -> String {
        String::new()
    }
}

#[derive(Hash, Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub timestamp: i64,
    pub hash: String,
    pub prev_hash: String,
    pub nonce: u64,
    pub transactions: Vec<Transaction>,
    pub data: String,
}


impl Chain {
    pub fn new() -> Self {
        Self { blocks: Vec::new(), difficulty: 0 }
    }

    pub fn gen(&mut self) {
        let gen_block = Block::new(
            0,
            Utc::now().timestamp_micros(),
            String::from("gen"),
            2836,
            Vec::new(),
            String::from("gen"),
        );
        self.blocks.push(gen_block);
    }

    pub fn to_binary_repr(&self) -> String {
        self.blocks
            .iter()
            .map(|block| block.hash_hex_to_binary_repr())
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn try_add_block(&mut self, block: Block) -> () {
        let latest = self.blocks.last()
            .expect("Could not get latest block");
        if self.is_valid_block(&block, latest) {
            self.blocks.push(block);
            log::info!("Added block {}", self.blocks.len() - 1);
        }
        log::error!("Could not add block - invalid")
    }

    pub fn is_valid_chain(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 { continue; }
            let fir = chain.get(i - 1).expect("Could not get first block");
            let sec = chain.get(i).expect("Could not get second block");
            if !self.is_valid_block(sec, fir) {
                return false;
            }
        }
        true
    }

    pub fn choose_chain(&mut self, loc: Vec<Block>, rem: Vec<Block>) -> Vec<Block> {
        if self.is_valid_chain(&loc) {
            if self.is_valid_chain(&rem) {
                if loc.len() > rem.len() {
                    return loc;
                } else {
                    return rem;
                }
            }
            return loc;
        } 
        if self.is_valid_chain(&rem) {
            return rem;
        }
        panic!("Both chains are invalid")
    }

    pub fn is_valid_block(&self, block: &Block, prev: &Block) -> bool {
        if block.prev_hash != prev.hash {
            log::warn!("block with id {} has invalid prev_hash", block.id);
            return false;
        } else if !Block::hash_to_binary_repr(
            &hex::decode(&block.hash).expect("can decode from hex"),
        ).starts_with(DIFFICULTY_PREFIX) {
            log::warn!("block with id {} has invalid difficulty", block.id);
            return false;
        } else if block.id != prev.id + 1 {
            log::warn!("block with id {} has invalid id for latest {}", block.id, prev.id);
            return false;
        } else if hex::encode(Block::calc_hash(
            block.id,
            block.timestamp,
            &block.prev_hash,
            &block.data,
            &block.transactions,
            block.nonce,
        )) != block.hash {
            log::warn!("block with id {} has invalid hash", block.id);
            return false;
        } else if block.timestamp <= prev.timestamp {
            log::warn!("block with id {} has invalid timestamp", block.id);
            return false;
        }
        true
    }
}

impl Block {
    pub fn new(
        id: u64,
        timestamp: i64,
        prev_hash: String,
        nonce: u64,
        transactions: Vec<Transaction>,
        data: String,
    ) -> Self {
        Self {
            id, timestamp,
            prev_hash,
            nonce,
            transactions,
            hash: String::new(),
            data,
        }
    }
    pub fn gen(
        id: u64, 
        prev_hash: String, 
        data: String, 
        transactions: Vec<Transaction>
    ) -> Self {
        let now = Utc::now().timestamp();
        let (nonce, hash) = Block::mine(id, now, &prev_hash, &data, &transactions);
        Self {
            id, hash, timestamp: now,
            transactions,
            prev_hash, data, nonce,
        }
    }
    
    pub fn mine(
        id: u64,
        timestamp: i64,
        prev_hash: &str,
        data: &str,
        transactions: &[Transaction],
    ) -> (u64, String) {
        log::info!("Miningb block {}", id);
        let mut nonce = 0;
        loop {
            if nonce % 100000 == 0 {
                log::info!("Nonce at {}", nonce);
            }
            let b= Block::new(id, timestamp, prev_hash.to_owned(), nonce, transactions.to_owned(), data.to_owned());
            let hash = b.get_hash();
            let binhash = b.hash_hex_to_binary_repr();
            if binhash.starts_with(DIFFICULTY_PREFIX) {
                log::info!("mined nonce {}, hash {}, binhash {}", nonce, hex::encode(&b.hash), binhash);
                return (nonce, hex::encode(hash))
            }
            nonce += 1;
        }
    }
    pub fn calc_hash(
        id: u64,
        timestamp: i64,
        prev_hash: &str,
        data: &str,
        transactions: &[Transaction],
        nonce: u64,
    ) -> Vec<u8> {
        let data = serde_json::json!({
            "id": id,
            "prev_hash": prev_hash,
            "data": data,
            "timestamp": timestamp,
            "nonce": nonce,
            "transactions": transactions,
        });
        // let reader = BufReader::new(data.to_string().as_bytes());
        // reader.buffer().to_vec()
        data.to_string().as_bytes().to_vec()
        // let digest = Self::sha256_digest(reader).unwrap();
        // digest.as_ref().to_vec()
    }
    pub fn sha256_digest<R: Read>(mut reader: R) -> std::io::Result<Digest> {
        let mut hasher = Context::new(&SHA256);
        let mut buf = [0; 1024];
        loop {
            let c = reader.read(&mut buf)?;
            if c == 0 { break; }
            hasher.update(&buf[..c]);
        }
        Ok(hasher.finish())

    }

    pub fn get_hash(&self) -> Vec<u8> {
        Self::calc_hash(
            self.id,
            self.timestamp,
            &self.prev_hash,
            &self.data,
            &self.transactions,
            self.nonce,
        )
    }
    
    pub fn txs_hash(&self) -> String {
        self.transactions
            .iter()
            .map(|tx| tx.to_string())
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn hash_str(&self) -> String {
        format!("{}{}{}{}{}",
            self.id,
            self.timestamp,
            self.prev_hash,
            self.nonce,
            self.txs_hash(),
        )
    }

    pub fn hash(&mut self) {
        self.hash = self.hash_str();
    }

    pub fn is_valid_hash(&self, b: &Block) -> bool {
        self.hash == self.hash_str()
    }

    pub fn hash_hex_to_binary_repr(self: &Self) -> String {
        Self::hash_to_binary_repr(&hex::decode(self.get_hash()).expect("can decode from hex"))
    }

    pub fn hash_to_binary_repr(hash: &[u8]) -> String {
        hash.iter()
            .map(|byte| format!("{:b}", byte))
            .collect::<Vec<String>>()
            .join("")
    }
}