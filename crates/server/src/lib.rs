use async_graphql::{EmptyMutation, EmptySubscription, ObjectType, Schema, http::GraphiQLSource};
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    response::{self, IntoResponse},
    routing::get,
};
use primitives::ServerConfig;
use tokio::net::TcpListener;

pub mod db_query;

/// This function is used to serve the graphQL server and GraphiQL IDE.
async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

/// This function is used to run the server.
/// `[DB]` This is a generic type, which is used to store the database.
/// `[Query]` This is a gaint Query entity, for all the Events enitities and all the tx enitities.
pub async fn run_server<Query>(
    config: ServerConfig,
    query: Query,
) -> Result<(), anyhow::Error>
where
    Query: ObjectType + 'static,
{
    let url = config.server_url.clone();
    let schema = Schema::build(query, EmptyMutation, EmptySubscription)
        .data(config)
        .finish();
    let app = Router::new().route("/", get(graphiql).post_service(GraphQL::new(schema)));

    tracing::info!(url);
    axum::serve(TcpListener::bind(url).await.unwrap(), app)
        .await
        .unwrap();

    Ok(())
}
