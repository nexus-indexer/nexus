use alloy::{
    primitives::{Address, Bytes, B256, U256},
    rpc::types::eth::{Log, Transaction},
};
use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};

use crate::db::{ DatabaseManager};


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChronicleEvent {
    pub address: Address,
    pub block_number: u64,
    pub block_hash: B256,           // Add this
    pub transaction_hash: B256,
    pub transaction_index: u64,     // Add this
    pub log_index: u64,             // Add this
    pub topics: Vec<B256>,
    pub data: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, SimpleObject)]
pub struct DisplayChronicleEvent {
    pub address: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub topics: Vec<String>,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChronicleTransaction {
    pub hash: B256,
    pub nonce: u64,
    pub block_hash: B256,
    pub block_number: u64,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub gas_price: u128,
    pub gas: u128,
    pub max_fee_per_gas: u128,
    pub data: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChronicleIndexingMode {
    Transaction,
    Event,
}

impl From<Transaction> for ChronicleTransaction {
    fn from(tx: Transaction) -> Self {
        Self {
            hash: tx.hash,
            nonce: tx.nonce,
            block_hash: tx.block_hash.unwrap_or(B256::default()),
            block_number: tx.block_number.unwrap_or(0),
            from: tx.from,
            to: tx.to.unwrap_or(Address::ZERO),
            value: tx.value,
            gas_price: tx.gas_price.unwrap_or(0),
            gas: tx.gas,
            data: tx.input,
            max_fee_per_gas: tx.max_fee_per_gas.unwrap_or(0),
        }
    }
}



impl From<Log> for ChronicleEvent {
    fn from(log: Log) -> Self {
        Self {
            address: log.address(),
            block_number: log.block_number.unwrap_or(0),
            block_hash: log.block_hash.unwrap_or_else(|| B256::ZERO),
            transaction_hash: log.transaction_hash.unwrap_or_else(|| B256::ZERO),
            transaction_index: log.transaction_index.map(|idx| idx as u64).unwrap_or(0),
            log_index: log.log_index.map(|idx| idx as u64).unwrap_or(0),
            topics: log.topics().to_vec(),
            data: log.inner.data.data,
        }
    }
}


impl DisplayChronicleEvent {
    pub fn new(
        address: String,
        block_number: String,
        transaction_hash: String,
        topics: Vec<String>,
        data: String,
    ) -> Self {
        Self {
            address,
            block_number,
            transaction_hash,
            topics,
            data,
        }
    }
}



impl ChronicleEvent {
    pub async fn store_event(
        &self,
        db_manager: &DatabaseManager, // Change parameter type
        event_name: &str,
    ) -> Result<(), anyhow::Error> {
        db_manager.store_event(event_name, self).await?;
        Ok(())
    }
}