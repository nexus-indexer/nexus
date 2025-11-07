
pub mod utils;

use self::utils::{query_events, subscribe_to_events};
use alloy::{
    primitives::{Address, B256},
    providers::RootProvider,
    pubsub::PubSubFrontend,
    rpc::types::eth::BlockNumberOrTag,
};
use async_trait::async_trait;
use chronicle_primitives::{
    db::DatabaseManager,
    interfaces::ChronicleEventIndexer,
};

pub struct EvmEventIndexer {
    name: String,
}

impl EvmEventIndexer {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl ChronicleEventIndexer for EvmEventIndexer {
    type SubProvider = RootProvider<PubSubFrontend>;
    type ContractAddress = Address;
    type EventSignature = B256;
    type BlockNumber = BlockNumberOrTag;

    async fn query_events(
        &self,
        provider: Self::SubProvider,
        addr: Self::ContractAddress,
        event_sig: Self::EventSignature,
        block_number: Self::BlockNumber,
        db_manager: &DatabaseManager, // Now matches the trait
    ) -> Result<(), anyhow::Error> {
        // Register the event indexer first (this creates the partitioned table)
        db_manager.register_event_indexer(
            &self.name,
            &addr.to_string(),
            &event_sig.to_string(),
            match block_number {
                BlockNumberOrTag::Number(n) => n,
                _ => 0, // Handle other cases appropriately
            },
        ).await?;

        // Query existing events from the specified block number
        let events = query_events(provider.clone(), addr, event_sig, block_number).await?;

        // Store all events in the database
        for event in events {
            db_manager.store_event(&self.name, &event).await?;
        }

        // Now subscribe to new events
        self.subscribe_to_events(provider, vec![addr], event_sig, db_manager)
            .await?;

        Ok(())
    }

    async fn subscribe_to_events(
        &self,
        provider: Self::SubProvider,
        addr: Vec<Self::ContractAddress>,
        event_sig: Self::EventSignature,
        db_manager: &DatabaseManager, // Now matches the trait
    ) -> Result<(), anyhow::Error> {
        subscribe_to_events(provider, addr, event_sig, db_manager, &self.name).await;
        Ok(())
    }
}