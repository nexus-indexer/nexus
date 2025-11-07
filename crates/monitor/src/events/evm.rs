use alloy::{
    dyn_abi::{DecodedEvent, DynSolEvent, DynSolType},
    network::Ethereum,
    primitives::{Address, B256, Bytes, LogData},
    providers::{Provider, RootProvider},
    rpc::types::eth::{BlockNumberOrTag, Filter},
};
use futures::StreamExt;
use primitives::{db::store_event_to_db, monitor::Event};

pub async fn query_events(
    provider: RootProvider<Ethereum>,
    addr: Address,
    event_sig: B256,
    block_number: BlockNumberOrTag,
) -> Result<Vec<Event>, anyhow::Error> {
    let filter = Filter::new()
        .address(addr)
        .event_signature(event_sig)
        .from_block(block_number);
    let log = provider.get_logs(&filter).await?;
    let chronicle_logs: Vec<Event> = log.into_iter().map(|log| log.into()).collect();

    Ok(chronicle_logs)
}

pub async fn subscribe_to_events(
    provider: RootProvider<Ethereum>,
    addr: Vec<Address>,
    event_sig: B256,
    client: &mut tokio_postgres::Client,
    name: &str,
) {
    let filter = Filter::new()
        .address(addr)
        .event_signature(event_sig)
        .from_block(BlockNumberOrTag::Latest);

    let sub = provider
        .subscribe_logs(&filter)
        .await
        .expect("Failed to subscribe to logs");
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        store_event_to_db(&log.into(), client, name)
            .await
            .expect("Failed to store event to db");
    }
}

pub fn decode_event(
    topics: Vec<B256>,
    data: Bytes,
    decoder_format: DynSolType,
    indexed: Vec<DynSolType>,
) -> Result<DecodedEvent, anyhow::Error> {
    let event: DynSolEvent = DynSolEvent::new_unchecked(Some(topics[0]), indexed, decoder_format);
    let log_data = LogData::new_unchecked(topics, data);
    let decoded_event = event.decode_log_data(&log_data)?;

    Ok(decoded_event)
}
