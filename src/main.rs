use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use clap::Parser;
use colored::Colorize;
use eyre::Result;
use tokio::sync::RwLock;

mod routes;
mod utils;

use utils::{HttpLogger, KeyJuggler, Logger};
use crate::utils::cli::Args;
use crate::utils::config::config;

#[derive(Clone)]
pub struct AppState {
    config: utils::Config,
    client: Arc<awc::Client>,
    juggler: Arc<RwLock<KeyJuggler>>,
}

impl AppState {
    fn new(config: utils::Config) -> Self {
        Self {
            config: config.clone(),
            #[allow(clippy::arc_with_non_send_sync)]
            client: Arc::new(awc::Client::builder().disable_timeout().finish()),
            juggler: Arc::new(RwLock::new(KeyJuggler::new(config.keys.clone()))),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    Logger::init(args.verbosity);

    let config = config(args.config)?;
    let (host, port) = (config.host.clone(), config.port);
    let num_keys = config.keys.len();

    log::info!("loaded {} api {}", num_keys.to_string().cyan().bold(), if num_keys == 1 { "key" } else { "keys" });

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header().max_age(3600))
            .wrap(HttpLogger)
            .app_data(web::Data::new(AppState::new(config.clone())))
            .service(routes::completion)
            .service(routes::openai_completion)
    })
    .bind((host, port))?
    .run()
    .await
    .map_err(Into::into)
}
