use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use clap::Parser;
use eyre::Result;
use tokio::sync::RwLock;
use utils::HttpLogger;
use utils::KeyJuggler;

use crate::utils::cli::Args;
use crate::utils::config::Config;
use crate::utils::config::config;

mod routes;
mod utils;

#[derive(Clone)]
pub struct AppState {
    config: Config,
    client: Arc<awc::Client>,
    juggler: Arc<RwLock<KeyJuggler>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    utils::Logger::init(args.verbosity);

    let config: Config = config(args.config)?;

    let server_host = config.host.clone();
    let server_port = config.port;

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .wrap(HttpLogger)
            .app_data(web::Data::new(AppState {
                config: config.clone(),
                client: Arc::new(awc::Client::builder().disable_timeout().finish()),
                juggler: Arc::new(RwLock::new(KeyJuggler::new(config.keys.clone()))),
            }))
            .service(routes::completion)
            .service(routes::openai_completion)
    })
    .bind((server_host, server_port))?
    .run()
    .await
    .map_err(Into::into)
}
