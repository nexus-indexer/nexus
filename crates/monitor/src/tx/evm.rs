use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{Block, Transaction, TransactionTrait},
};
use futures::StreamExt;

pub async fn subscribe_transactions<F>(
    index_address: Address,
    ws_url: &str,
    mut callback: F,
) -> Result<(), anyhow::Error>
where
    F: FnMut(Vec<Transaction>),
{
    // Create WebSocket provider
    let ws = WsConnect::new(ws_url);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // Subscribe to new blocks
    let subscription = provider.subscribe_blocks().await?;
    let mut stream = subscription.into_stream();

    while let Some(header) = stream.next().await {
        // Fetch the full block with transactions
        let full_block = provider
            .get_block_by_number(header.inner.number.into())
            .await?;

        if let Some(Block {
            transactions: alloy::rpc::types::BlockTransactions::Full(txs),
            ..
        }) = full_block
        {
            let filtered_txs = txs
                .into_iter()
                .filter(|tx| {
                    println!("Captured Tx: {:?}", tx.block_hash);

                    // Get the 'to' address, defaulting to zero address if None
                    let to_address = tx.inner.to().unwrap_or(Address::ZERO);

                    // Filter transactions involving the target address
                    tx.inner.signer() == index_address || to_address == index_address
                })
                .collect::<Vec<Transaction>>();

            if !filtered_txs.is_empty() {
                callback(filtered_txs);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use alloy::primitives::address;

    #[tokio::test]
    #[ignore]
    pub async fn test_subscribe_transactions_works() {
        let rpc_url = "wss://ethereum-rpc.publicnode.com";
        let usdc_token_address = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

        let callback = |tx: Vec<Transaction>| {
            println!("Received Tx: {:?}", tx);
        };

        subscribe_transactions(usdc_token_address, rpc_url, callback)
            .await
            .unwrap();
    }
}
