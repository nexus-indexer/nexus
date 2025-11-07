
use async_graphql::{Context, Object};
use chronicle_primitives::{
    db::DatabaseManager,
    indexer::DisplayChronicleEvent,
    ServerConfig,
};

pub struct ChronicleQuery;

#[Object]
impl ChronicleQuery {
    async fn get_all_events<'a>(
        &self,
        cxt: &Context<'a>,
        name: String,
    ) -> Vec<DisplayChronicleEvent> {
        let config = cxt.data_unchecked::<ServerConfig>();
        let db_manager = DatabaseManager::new(&config.db_url)
            .await
            .expect("Could not connect to the db");

        let events = db_manager.get_all_events(&name)
            .await
            .expect("Could not get events from db");

        events
    }

    async fn get_events_by_tx_hash<'a>(
        &self,
        cxt: &Context<'a>,
        name: String,
        transaction_hash: String,
    ) -> Vec<DisplayChronicleEvent> {
        let config = cxt.data_unchecked::<ServerConfig>();
        let db_manager = DatabaseManager::new(&config.db_url)
            .await
            .expect("Could not connect to the db");

        let events = db_manager.get_events_by_tx_hash(&name, transaction_hash)
            .await
            .expect("Could not get events from db");

        events
    }

    async fn get_events_by_block_number<'a>(
        &self,
        cxt: &Context<'a>,
        name: String,
        block_number: String,
    ) -> Vec<DisplayChronicleEvent> {
        let config = cxt.data_unchecked::<ServerConfig>();
        let db_manager = DatabaseManager::new(&config.db_url)
            .await
            .expect("Could not connect to the db");

        let events = db_manager.get_events_by_block_number(&name, block_number)
            .await
            .expect("Could not get events from db");

        events
    }

    // Add new query methods for the enhanced functionality
    async fn get_events_by_range<'a>(
        &self,
        cxt: &Context<'a>,
        name: String,
        from_block: u64,
        to_block: u64,
        limit: Option<i64>,
    ) -> Vec<DisplayChronicleEvent> {
        let config = cxt.data_unchecked::<ServerConfig>();
        let db_manager = DatabaseManager::new(&config.db_url)
            .await
            .expect("Could not connect to the db");

        let events = db_manager.get_events_by_range(&name, from_block, to_block, limit)
            .await
            .expect("Could not get events from db");

        events
    }
}