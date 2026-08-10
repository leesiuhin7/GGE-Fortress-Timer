use std::{env, sync::Arc};

use axum::{
    Router,
    http::{Response, StatusCode},
    routing::get,
};
use tracing::error;
use tracing_subscriber::EnvFilter;

use crate::{
    manager::{Error, Manager},
    serialize::serialize_locations,
    storage::Storage,
};

mod client;
mod config;
mod core;
mod manager;
mod map;
mod serialize;
mod storage;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let uri = env::var("DB_URI").expect("DB_URI is required");
    let storage = Arc::new(Storage::connect(&uri).await.unwrap());

    let app = Router::new()
        .route(
            "/data",
            get({
                let storage = Arc::clone(&storage);
                async move || match storage.get_locations().await {
                    Ok(locations) => Response::new(serialize_locations(locations)),
                    Err(_) => Response::builder().status(500).body(String::new()).unwrap(),
                }
            }),
        )
        .route("/keep-alive", get(async || StatusCode::OK));

    let port = env::var("PORT").unwrap_or("10000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    let serve = axum::serve(listener, app);

    let manager = Manager::new(&storage);
    let log_error = |result| match result {
        Err(Error::Config(error)) => error!(%error, "Failed to load Config"),
        Err(Error::Locations(error)) => error!(%error, "Failed to load Locations"),
        Err(Error::ExplorerItems(error)) => error!(%error, "Failed to load ExplorerItems"),
        Ok(()) => (),
    };

    tokio::select! {
        _ = serve => (),
        result = manager.run() => log_error(result),
    }
}
