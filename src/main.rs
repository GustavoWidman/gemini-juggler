use std::sync::{Arc, RwLock};

use actix_web::{App, HttpServer, web};
use easy_config_store::ConfigStore;
use log::LevelFilter;
use utils::HttpLogger;
use utils::KeyJuggler;

mod config;
mod routes;
mod utils;

#[derive(Clone)]
pub struct AppState {
    config: Arc<ConfigStore<config::Config>>,
    client: Arc<awc::Client>,
    juggler: Arc<RwLock<KeyJuggler>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    utils::Logger::init(LevelFilter::Info);

    let config: Arc<ConfigStore<config::Config>> = Arc::new(
        ConfigStore::read("config.toml")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
    );

    let server_host = config.host.clone();
    let server_port = config.port;

    HttpServer::new(move || {
        App::new()
            .wrap(HttpLogger)
            .app_data(web::Data::new(AppState {
                config: config.clone(),
                client: Arc::new(awc::Client::builder().disable_timeout().finish()),
                juggler: Arc::new(RwLock::new(KeyJuggler::new(config.keys.clone()))),
            }))
            .service(routes::completion)
            .service(routes::openai_completion)
        // .service(serve::download)
        // .service(serve::login)
    })
    .bind((server_host, server_port))?
    .run()
    .await
}
