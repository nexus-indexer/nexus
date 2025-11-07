// use std::result::Result::Ok;
// use crate::Task;
// use alloy::{providers::ProviderBuilder, rpc::client::WsConnect};
// use anyhow::{bail};
// use async_trait::async_trait;
// use chronicle_indexer::events::evm::EvmEventIndexer;
// use chronicle_primitives::{
//     db::create_db_instance, interfaces::ChronicleEventIndexer, IndexerConfig, StateMachine,
// };
// use tokio::{select, try_join};
// use tokio_util::sync::CancellationToken;
// use tracing::info;
//
// #[derive(Debug)]
// pub struct IndexerTask {
//     pub config: IndexerConfig,
// }
//
// #[async_trait]
// impl Task for IndexerTask {
//     async fn run(mut self: Box<Self>, shutdown_token: CancellationToken) -> anyhow::Result<()> {
//         let mut client = create_db_instance(&self.config.db_url)
//             .await
//             .expect("Could not create db instance");
//         let ws = WsConnect::new(self.config.rpc_url.clone());
//         let provider = ProviderBuilder::new().on_ws(ws).await?;
//         match self.config.state_machine.clone().into() {
//             StateMachine::EVM => {
//                 let evm_event_indexer = EvmEventIndexer::new(self.config.event_name.clone());
//
//                 // This queries events that have happened from this block number and stores them in the database
//                 // It also subscribes to new events and stores them in the database
//                 let evm_indexer_handle = tokio::spawn(async move {
//                     select! {
//                         event_n_sub = evm_event_indexer.query_events(
//                             provider.clone(),
//                             self.config.address.clone().parse().expect("CONFIG address could not be parsed"),
//                             self.config.event_signature.clone().parse().expect("CONFIG event signature is missing"),
//                             self.config.block_number.into(),
//                             &mut client,
//                         ) => {
//                             // Want this indexing to halt before
//                             if event_n_sub.is_err() {
//                                 info!("Event subscription error, exiting now. ERROR: {:?}", event_n_sub.err().unwrap());
//                             }
//                         }
//                         _ = shutdown_token.cancelled() => {
//                             info!("Shutting down chronicle indexer");
//                         }
//                     }
//                 });
//
//                 match try_join!(evm_indexer_handle) {
//                     Ok(_) => {
//                         info!("Server task completed");
//                     }
//                     Err(e) => bail!("Error running server: {:?}", e),
//                 }
//             }
//             StateMachine::PARACHAIN => {}
//         }
//         Ok(())
//     }
// }
//
// impl IndexerTask {
//     pub fn new(config: IndexerConfig) -> Self {
//         Self { config }
//     }
//
//     /// Converts the task into a boxed trait object.
//     pub fn boxed(self) -> Box<dyn Task> {
//         Box::new(self)
//     }
// }




use crate::Task;
use std::sync::Arc;
use alloy::{providers::ProviderBuilder, rpc::client::WsConnect};
use anyhow::bail;
use async_trait::async_trait;
use chronicle_indexer::events::evm::EvmEventIndexer;
use chronicle_primitives::{
    db::DatabaseManager,
    interfaces::ChronicleEventIndexer,
    IndexerConfig,
    StateMachine,
};
use tokio::{select, try_join};
use tokio_util::sync::CancellationToken;
use tracing::{info, error};

#[derive(Debug, Clone)] // Add Clone trait
pub struct IndexerTask {
    pub config: IndexerConfig,
    pub db_manager: Arc<DatabaseManager>,
}

#[async_trait]
impl Task for IndexerTask {
    async fn run(mut self: Box<Self>, shutdown_token: CancellationToken) -> anyhow::Result<()> {
        // Register the indexer first
        if let Err(e) = self.db_manager.register_event_indexer(
            &self.config.event_name,
            &self.config.address,
            &self.config.event_signature,
            self.config.block_number,
        ).await {
            error!("Failed to register indexer {}: {}", self.config.event_name, e);
            return Err(e);
        }

        let ws = WsConnect::new(self.config.rpc_url.clone());
        let provider = ProviderBuilder::new().on_ws(ws).await?;

        match self.config.state_machine.clone().into() {
            StateMachine::EVM => {
                // Clone the data needed for the spawned task
                let event_name = self.config.event_name.clone();
                let address = self.config.address.clone();
                let event_signature = self.config.event_signature.clone();
                let block_number = self.config.block_number;
                let db_manager = self.db_manager.clone(); // Clone the DatabaseManager

                let evm_event_indexer = EvmEventIndexer::new(event_name.clone());

                let evm_indexer_handle = tokio::spawn(async move {
                    select! {
                        result = evm_event_indexer.query_events(
                            provider.clone(),
                            address.parse().expect("Invalid address"),
                            event_signature.parse().expect("Invalid event signature"),
                            block_number.into(),
                            &db_manager, // Use the cloned DatabaseManager
                        ) => {
                            if let Err(e) = result {
                                error!("Event subscription error for {}: {:?}", event_name, e);
                            }
                        }
                        _ = shutdown_token.cancelled() => {
                            info!("Shutting down indexer for: {}", event_name);
                        }
                    }
                });

                match try_join!(evm_indexer_handle) {
                    Ok(_) => {
                        info!("Indexer task completed for: {}", self.config.event_name);
                    }
                    Err(e) => {
                        error!("Error running indexer for {}: {:?}", self.config.event_name, e);
                        bail!("Indexer error: {:?}", e);
                    }
                }
            }
            StateMachine::PARACHAIN => {
                info!("Parachain indexing not yet implemented for: {}", self.config.event_name);
            }
        }
        Ok(())
    }
}

impl IndexerTask {
    pub async fn new(config: IndexerConfig) -> anyhow::Result<Self> {
        let db_manager = DatabaseManager::new(&config.db_url).await?;
        Ok(Self { config, db_manager: Arc::from(db_manager) })
    }

    pub fn boxed(self) -> Box<dyn Task> {
        Box::new(self)
    }
}