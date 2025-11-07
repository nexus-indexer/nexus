use alloy::hex;
use tokio_postgres::{Client, NoTls, Row};
use anyhow::{Result, anyhow};
use crate::indexer::{ChronicleEvent, DisplayChronicleEvent};

#[derive(Debug)]
pub struct DatabaseManager {
    client: Client,
}

impl DatabaseManager {
    pub async fn new(connection_string: &str) -> Result<Self> {
        // First, ensure the database exists
        Self::ensure_database_exists(connection_string).await?;

        // Now connect to the actual database
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("database connection error: {}", e);
            }
        });

        let manager = Self { client };
        manager.initialize_schema().await?;

        Ok(manager)
    }

    async fn ensure_database_exists(connection_string: &str) -> Result<()> {
        // Extract database name from connection string
        let db_name = Self::extract_db_name(connection_string)
            .ok_or_else(|| anyhow!("Could not extract database name from connection string"))?;

        // Create connection to default 'postgres' database
        let admin_connection_string = Self::replace_db_name(connection_string, "postgres");

        match tokio_postgres::connect(&admin_connection_string, NoTls).await {
            Ok((admin_client, connection)) => {
                // Spawn the connection
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("admin connection error: {}", e);
                    }
                });

                // Check if database exists
                let db_exists: bool = admin_client
                    .query_one(
                        "SELECT 1 FROM pg_database WHERE datname = $1",
                        &[&db_name]
                    )
                    .await
                    .is_ok();

                if !db_exists {
                    println!("Creating database: {}", db_name);
                    // Note: CREATE DATABASE cannot be executed in a transaction block
                    admin_client
                        .execute(&format!("CREATE DATABASE {}", db_name), &[])
                        .await?;
                    println!("Database '{}' created successfully", db_name);
                }
            }
            Err(e) => {
                // If we can't connect to admin database, just log and continue
                // The main connection might still work if the database already exists
                eprintln!("Warning: Could not connect to admin database to check/create '{}': {}", db_name, e);
                println!("Attempting to connect directly to database '{}'...", db_name);
            }
        }

        Ok(())
    }

    fn extract_db_name(connection_string: &str) -> Option<String> {
        if let Some(pos) = connection_string.find("dbname=") {
            let dbname_start = pos + 7; // "dbname=" is 7 chars
            let rest = &connection_string[dbname_start..];
            let dbname_end = rest.find(' ').unwrap_or(rest.len());
            Some(rest[..dbname_end].to_string())
        } else {
            None
        }
    }

    fn replace_db_name(connection_string: &str, new_db_name: &str) -> String {
        if let Some(pos) = connection_string.find("dbname=") {
            let dbname_start = pos + 7;
            let rest = &connection_string[dbname_start..];
            let dbname_end = rest.find(' ').unwrap_or(rest.len());
            format!(
                "{}dbname={}{}",
                &connection_string[..pos],
                new_db_name,
                &rest[dbname_end..]
            )
        } else {
            format!("{} dbname={}", connection_string, new_db_name)
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    async fn initialize_schema(&self) -> Result<()> {
        // Create metadata table
        self.client.batch_execute(
            r#"
            CREATE TABLE IF NOT EXISTS chronicle_metadata (
                id SERIAL PRIMARY KEY,
                event_name VARCHAR(100) NOT NULL UNIQUE,
                contract_address VARCHAR(42) NOT NULL,
                event_signature VARCHAR(66) NOT NULL,
                start_block BIGINT NOT NULL,
                current_block BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                enabled BOOLEAN DEFAULT true
            )
            "#
        ).await?;

        // Create indexes for metadata
        self.client.batch_execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_metadata_event_name ON chronicle_metadata (event_name);
            CREATE INDEX IF NOT EXISTS idx_metadata_contract ON chronicle_metadata (contract_address);
            "#
        ).await?;

        Ok(())
    }

    pub async fn register_event_indexer(
        &self,
        event_name: &str,
        contract_address: &str,
        event_signature: &str,
        start_block: u64,
    ) -> Result<()> {
        self.client.execute(
            r#"
            INSERT INTO chronicle_metadata
            (event_name, contract_address, event_signature, start_block, current_block)
            VALUES ($1, $2, $3, $4, $4)
            ON CONFLICT (event_name)
            DO UPDATE SET
                contract_address = EXCLUDED.contract_address,
                event_signature = EXCLUDED.event_signature,
                start_block = EXCLUDED.start_block,
                updated_at = NOW()
            "#,
            &[&event_name, &contract_address, &event_signature, &(start_block as i64)],
        ).await?;

        self.create_partitioned_event_table(event_name).await?;
        Ok(())
    }

    pub async fn create_partitioned_event_table(&self, event_name: &str) -> Result<()> {
        let table_name = Self::sanitize_table_name(event_name);
        let master_table = format!("events_{}", table_name);

        // Create master partitioned table
        self.client.batch_execute(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGSERIAL,
                address VARCHAR(42) NOT NULL,
                block_number BIGINT NOT NULL,
                block_hash VARCHAR(66),
                transaction_hash VARCHAR(66) NOT NULL,
                transaction_index INTEGER,
                log_index INTEGER NOT NULL,
                topics TEXT[] NOT NULL,
                data TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                PRIMARY KEY (block_number, log_index, transaction_hash)
            ) PARTITION BY RANGE (block_number)
            "#, master_table
        )).await?;

        // Create partitions for years 2020-2024
        let years = [2020, 2021, 2022, 2023, 2024];
        for &year in &years {
            let partition_start = (year * 10_000_000) as i64;
            let partition_end = ((year + 1) * 10_000_000) as i64;

            let partition_table = format!("{}_{}", master_table, year);

            self.client.batch_execute(&format!(
                r#"
                CREATE TABLE IF NOT EXISTS {} PARTITION OF {}
                FOR VALUES FROM ({}) TO ({})
                "#,
                partition_table, master_table, partition_start, partition_end
            )).await?;

            // Create indexes on partition
            self.client.batch_execute(&format!(
                r#"
                CREATE INDEX IF NOT EXISTS idx_{}_tx_hash ON {} (transaction_hash);
                CREATE INDEX IF NOT EXISTS idx_{}_address ON {} (address);
                CREATE INDEX IF NOT EXISTS idx_{}_topics ON {} USING GIN (topics);
                "#,
                partition_table, partition_table,
                partition_table, partition_table,
                partition_table, partition_table
            )).await?;
        }

        // Create default partition for future blocks
        let default_start = (2025 * 10_000_000) as i64;
        self.client.batch_execute(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}_future PARTITION OF {}
            FOR VALUES FROM ({}) TO (MAXVALUE)
            "#,
            master_table, master_table, default_start
        )).await?;

        Ok(())
    }

    pub async fn store_event(
        &self,
        event_name: &str,
        event: &ChronicleEvent,
    ) -> Result<()> {
        let table_name = Self::sanitize_table_name(event_name);
        let master_table = format!("events_{}", table_name);

        let stringified_topics: Vec<String> = event
            .topics
            .iter()
            .map(|topic| format!("0x{}", hex::encode(topic)))
            .collect();

        self.client.execute(
            &format!(
                r#"
            INSERT INTO {}
            (address, block_number, block_hash, transaction_hash, transaction_index, log_index, topics, data)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (block_number, log_index, transaction_hash) DO NOTHING
            "#,
                master_table
            ),
            &[
                &event.address.to_string(),
                &(event.block_number as i64),
                &event.block_hash.to_string(),                    // Use actual value
                &event.transaction_hash.to_string(),
                &(event.transaction_index as i32),               // Use actual value
                &(event.log_index as i32),                       // Use actual value
                &stringified_topics,
                &event.data.to_string(),
            ],
        ).await?;

        // Update current block in metadata
        self.client.execute(
            r#"
        UPDATE chronicle_metadata
        SET current_block = GREATEST(current_block, $2), updated_at = NOW()
        WHERE event_name = $1
        "#,
            &[&event_name, &(event.block_number as i64)],
        ).await?;

        Ok(())
    }


    pub async fn get_all_events(
        &self,
        event_name: &str,
    ) -> Result<Vec<DisplayChronicleEvent>> {
        let table_name = Self::sanitize_table_name(event_name);
        let master_table = format!("events_{}", table_name);

        let rows = self.client.query(
            &format!("SELECT * FROM {} ORDER BY block_number DESC, log_index DESC", master_table),
            &[],
        ).await?;

        Self::rows_to_display_events(rows)
    }

    pub async fn get_events_by_tx_hash(
        &self,
        event_name: &str,
        transaction_hash: String,
    ) -> Result<Vec<DisplayChronicleEvent>> {
        let table_name = Self::sanitize_table_name(event_name);
        let master_table = format!("events_{}", table_name);

        let rows = self.client.query(
            &format!("SELECT * FROM {} WHERE transaction_hash = $1", master_table),
            &[&transaction_hash],
        ).await?;

        Self::rows_to_display_events(rows)
    }

    pub async fn get_events_by_block_number(
        &self,
        event_name: &str,
        block_number: String,
    ) -> Result<Vec<DisplayChronicleEvent>> {
        let table_name = Self::sanitize_table_name(event_name);
        let master_table = format!("events_{}", table_name);

        let rows = self.client.query(
            &format!("SELECT * FROM {} WHERE block_number = $1", master_table),
            &[&block_number],
        ).await?;

        Self::rows_to_display_events(rows)
    }

    pub async fn get_events_by_range(
        &self,
        event_name: &str,
        from_block: u64,
        to_block: u64,
        limit: Option<i64>,
    ) -> Result<Vec<DisplayChronicleEvent>> {
        let table_name = Self::sanitize_table_name(event_name);
        let master_table = format!("events_{}", table_name);

        let query = format!(
            "SELECT * FROM {} WHERE block_number BETWEEN $1 AND $2 ORDER BY block_number DESC, log_index DESC LIMIT $3",
            master_table
        );

        let rows = self.client.query(
            &query,
            &[&(from_block as i64), &(to_block as i64), &limit.unwrap_or(1000)],
        ).await?;

        Self::rows_to_display_events(rows)
    }

    fn rows_to_display_events(rows: Vec<Row>) -> Result<Vec<DisplayChronicleEvent>> {
        let mut events = Vec::new();
        for row in rows {
            let address: String = row.get("address");
            let block_number: i64 = row.get("block_number");
            let transaction_hash: String = row.get("transaction_hash");
            let topics: Vec<String> = row.get("topics");
            let data: String = row.get("data");

            events.push(DisplayChronicleEvent::new(
                address,
                block_number.to_string(),
                transaction_hash,
                topics,
                data,
            ));
        }
        Ok(events)
    }

    pub async fn get_latest_block(&self, event_name: &str) -> Result<u64> {
        let row = self.client.query_one(
            "SELECT current_block FROM chronicle_metadata WHERE event_name = $1",
            &[&event_name],
        ).await?;

        let block_number: i64 = row.get(0);
        Ok(block_number as u64)
    }

    fn sanitize_table_name(name: &str) -> String {
        name.replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_")
            .to_lowercase()
    }
}

// Keep your existing create_db_instance for backward compatibility
pub async fn create_db_instance(url: &str) -> Result<Client, anyhow::Error> {
    let (client, connection) = tokio_postgres::connect(url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}



#[cfg(test)]
pub mod tests {
    use super::*;

    const DB_URL: &str = "host=localhost user=postgres password=@Tiptop2059! dbname=chronicle_events port=5432";

    #[tokio::test]
    #[ignore]
    pub async fn test_can_create_partitioned_table() {
        let db_manager = DatabaseManager::new(DB_URL).await.expect("Could not create db manager");
        let result = db_manager.register_event_indexer(
            "test_event",
            "0x1234567890123456789012345678901234567890",
            "0xtest123",
            1000,
        ).await;
        assert!(result.is_ok());
    }

    // Add more tests as needed...
}
