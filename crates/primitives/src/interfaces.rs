use crate::monitor::Tx;
use async_trait::async_trait;

/// The purpose of event monitoring triat is that it would be shared across many supported chains
#[async_trait]
pub trait EventMonitor {
    type SubProvider;
    type ContractAddress;
    type EventSignature;
    type BlockNumber;

    /// The purpose of this function is to querry events from a specified clock number
    /// Then `[Filter]` which would have `address`, `last_block` and event_signature as the parameters
    async fn query_events(
        &self,
        provider: Self::SubProvider,
        addr: Self::ContractAddress,
        event_sig: Self::EventSignature,
        block_nuber: Self::BlockNumber,
        db_client: &mut tokio_postgres::Client,
    ) -> Result<(), anyhow::Error>;

    /// The end goal of this function would be to  create a filter and then subscribes to an event returning the event
    /// stream <T: Stream<Item = Resp> + Unpin>
    async fn subscribe_to_events(
        &self,
        provider: Self::SubProvider,
        addr: Vec<Self::ContractAddress>,
        event_sig: Self::EventSignature,
        db_client: &mut tokio_postgres::Client,
    ) -> Result<(), anyhow::Error>;
}

/// This transaction tx monitor (indexer) trait is planned to be used across many supported chains

#[async_trait]
pub trait TransactionMonitor {
    type SubProvider;
    type TargetAddress;

    /// The purpose of the function would subscribe to blocks and filters trasactions based on the index address.
    /// Uses a callback closure to output the filter tx
    async fn subscribe_transactions<F>(
        &self,
        index_address: Self::TargetAddress,
        provider: Self::SubProvider,
        callback: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(Vec<Tx>);
}
