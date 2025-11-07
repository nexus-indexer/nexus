use crate::Task;
use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::client::WsConnect,
};
use anyhow::bail;
use async_trait::async_trait;
use monitor::events::EventMonitorTable;
use primitives::{MonitorConfig, db::create_db_instance, traits::EventMonitor};
use tokio::{select, try_join};
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Debug)]
pub struct MonitorTask {
    config: MonitorConfig,
}

#[async_trait]
impl Task for MonitorTask {
    async fn run(mut self: Box<Self>, shutdown_token: CancellationToken) -> anyhow::Result<()> {
        let mut client = create_db_instance(&self.config.db_url)
            .await
            .expect("Could not create db instance");
        let ws = WsConnect::new(self.config.rpc_url.clone());
        let provider = ProviderBuilder::new().connect_ws(ws).await?;
        let evm_event_indexer = EventMonitorTable::new(self.config.event_name.clone());

        // This queries events that have happened from this block number and stores them in the database
        // It also subscribes to new events and stores them in the database
        let evm_indexer_handle = tokio::spawn(async move {
            select! {
                event_n_sub = evm_event_indexer.query_and_subscribe_to_events(
                    provider.root().clone(),
                    self.config.address.clone().parse().expect("CONFIG address could not be parsed"),
                    self.config.event_signature.clone().parse().expect("CONFIG event signature is missing"),
                    self.config.block_number.into(),
                    &mut client,
                ) => {
                    // Want this indexing to halt before
                    if event_n_sub.is_err() {
                        info!("Event subscription error, exitting now. ERROR: {:?}", event_n_sub.err().unwrap());
                    }
                }
                _ = shutdown_token.cancelled() => {
                    info!("Shutting down chronicle indexer");
                }
            }
        });

        match try_join!(evm_indexer_handle) {
            Ok(_) => {
                info!("Server task completed");
            }
            Err(e) => bail!("Error running server: {:?}", e),
        }
        Ok(())
    }
}

impl MonitorTask {
    pub fn new(config: MonitorConfig) -> Self {
        Self { config }
    }

    /// Converts the task into a boxed trait object.
    pub fn boxed(self) -> Box<dyn Task> {
        Box::new(self)
    }
}
