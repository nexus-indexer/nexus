use crate::db::store_event_to_db;
use alloy::{
    primitives::{Address, B256, Bytes, U256},
    rpc::types::eth::{Log, Transaction, TransactionTrait},
};
use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};

// Event => Struct for monitoring events
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Event {
    pub address: Address,
    pub block_number: u64,
    pub transaction_hash: B256,
    pub topics: Vec<B256>,
    pub data: Bytes,
}

// DisplayEvent => Struct to display events
#[derive(Serialize, Deserialize, Debug, Clone, Default, SimpleObject)]
pub struct DisplayEvent {
    pub address: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub topics: Vec<String>,
    pub data: String,
}

// Monitoring and getting of the Transactions
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Tx {
    pub hash: B256,
    pub nonce: u64,
    pub block_hash: B256,
    pub block_number: u64,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub data: Bytes,
}

// The struct to index for the event and for transactions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexingMode {
    Transaction,
    Event,
}

impl From<Transaction> for Tx {
    fn from(tx: Transaction) -> Self {
        Self {
            hash: *tx.inner.tx_hash(),
            nonce: tx.nonce(),
            block_hash: tx.block_hash.unwrap_or(B256::default()),
            block_number: tx.block_number.unwrap_or(0),
            from: tx.inner.signer(),
            to: tx.inner.to().unwrap_or(Address::ZERO),
            value: tx.value(),
            gas_price: tx.gas_price().unwrap_or(0),
            gas_limit: tx.gas_limit(),
            data: tx.input().clone(),
            max_fee_per_gas: tx.max_fee_per_gas(),
        }
    }
}

impl From<Log> for Event {
    fn from(log: Log) -> Self {
        Self {
            address: log.address(),
            block_number: log.block_number.unwrap_or(0),
            transaction_hash: log.transaction_hash.unwrap_or(B256::default()),
            topics: log.data().clone().topics().to_vec(),
            data: log.inner.data.data,
        }
    }
}

impl DisplayEvent {
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

impl Event {
    pub async fn store_event(
        &self,
        db_client: &mut tokio_postgres::Client,
        name: &str,
    ) -> Result<(), anyhow::Error> {
        store_event_to_db(self, db_client, name).await?;

        Ok(())
    }
}
