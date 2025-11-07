// // pub mod utils;
// //
// // use self::utils::{query_events, subscribe_to_events};
// // use alloy::{
// //     primitives::{Address, B256},
// //     providers::RootProvider,
// //     pubsub::PubSubFrontend,
// //     rpc::types::eth::BlockNumberOrTag,
// // };
// // use async_trait::async_trait;
// // use chronicle_primitives::{
// //     db::{create_new_event_db_table, store_event_to_db},
// //     interfaces::ChronicleEventIndexer,
// // };
// //
// // pub struct EvmEventIndexer {
// //     /// This is the name if this indexer instance, this is used for the DB table name
// //     name: String,
// // }
// //
// // impl EvmEventIndexer {
// //     pub fn new(name: String) -> Self {
// //         Self { name }
// //     }
// // }
// //
// // #[async_trait]
// // impl ChronicleEventIndexer for EvmEventIndexer {
// //     type SubProvider = RootProvider<PubSubFrontend>;
// //     type ContractAddress = Address;
// //     type EventSignature = B256;
// //     type BlockNumber = BlockNumberOrTag;
// //
// //     // TODO: This should be renamed to query then subscribe events
// //     async fn query_events(
// //         &self,
// //         provider: Self::SubProvider,
// //         addr: Self::ContractAddress,
// //         event_sig: Self::EventSignature,
// //         block_number: Self::BlockNumber,
// //         db_client: &mut tokio_postgres::Client,
// //     ) -> Result<(), anyhow::Error> {
// //         create_new_event_db_table(db_client, &self.name).await?;
// //         // Query existing events from the specified block number
// //         let events = query_events(provider.clone(), addr, event_sig, block_number).await?;
// //
// //         // Store all this event is the database
// //         for event in events {
// //             store_event_to_db(&event, db_client, &self.name).await?;
// //         }
// //
// //         // Now subscribing the events
// //         self.subscribe_to_events(provider, vec![addr], event_sig, db_client)
// //             .await?;
// //
// //         Ok(())
// //     }
// //
// //     async fn subscribe_to_events(
// //         &self,
// //         provider: Self::SubProvider,
// //         addr: Vec<Self::ContractAddress>,
// //         event_sig: Self::EventSignature,
// //         db_client: &mut tokio_postgres::Client,
// //     ) -> Result<(), anyhow::Error> {
// //         subscribe_to_events(provider, addr, event_sig, db_client, &self.name).await;
// //
// //         Ok(())
// //     }
// // }
//
//
// pub mod utils;
//
// use self::utils::{query_events, subscribe_to_events};
// use alloy::{
//     primitives::{Address, B256},
//     providers::RootProvider,
//     pubsub::PubSubFrontend,
//     rpc::types::eth::BlockNumberOrTag,
// };
// use async_trait::async_trait;
// use chronicle_primitives::{
//     db::DatabaseManager,
//     interfaces::ChronicleEventIndexer,
// };
//
// pub struct EvmEventIndexer {
//     name: String,
// }
//
// impl EvmEventIndexer {
//     pub fn new(name: String) -> Self {
//         Self { name }
//     }
// }
//
// #[async_trait]
// impl ChronicleEventIndexer for EvmEventIndexer {
//     type SubProvider = RootProvider<PubSubFrontend>;
//     type ContractAddress = Address;
//     type EventSignature = B256;
//     type BlockNumber = BlockNumberOrTag;
//     type DbManager = DatabaseManager; // Changed from Client to DatabaseManager
//
//     async fn query_events(
//         &self,
//         provider: Self::SubProvider,
//         addr: Self::ContractAddress,
//         event_sig: Self::EventSignature,
//         block_number: Self::BlockNumber,
//         db_manager: &Self::DbManager, // Changed parameter type
//     ) -> Result<(), anyhow::Error> {
//         // Table creation is now handled by DatabaseManager during registration
//
//         // Query existing events from the specified block number
//         let events = query_events(provider.clone(), addr, event_sig, block_number).await?;
//
//         // Store all events in the database using the new store_event method
//         for event in events {
//             db_manager.store_event(&self.name, &event).await?;
//         }
//
//         // Now subscribe to new events
//         self.subscribe_to_events(provider, vec![addr], event_sig, db_manager)
//             .await?;
//
//         Ok(())
//     }
//
//     async fn subscribe_to_events(
//         &self,
//         provider: Self::SubProvider,
//         addr: Vec<Self::ContractAddress>,
//         event_sig: Self::EventSignature,
//         db_manager: &Self::DbManager, // Changed parameter type
//     ) -> Result<(), anyhow::Error> {
//         // You'll need to update the subscribe_to_events function in utils
//         // to accept DatabaseManager instead of Client
//         subscribe_to_events(provider, addr, event_sig, db_manager, &self.name).await;
//         Ok(())
//     }
// }




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