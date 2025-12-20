use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use clap::Parser;
use eyre::Result;
use log::info;
use tokio::sync::RwLock;

mod routes;
mod utils;

use crate::utils::cli::Args;
use crate::utils::config::config;
use utils::{HttpLogger, KeyJuggler, Logger};

#[derive(Clone)]
pub struct AppState {
    config: utils::Config,
    client: Arc<awc::Client>,
    juggler: Arc<RwLock<KeyJuggler>>,
}

impl AppState {
    fn new(config: utils::Config, juggler: Arc<RwLock<KeyJuggler>>) -> Self {
        Self {
            config: config.clone(),
            #[allow(clippy::arc_with_non_send_sync)]
            client: Arc::new(awc::Client::builder().disable_timeout().finish()),
            juggler,
        }
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    Logger::init(args.verbosity);

    let config = config(args.config)?;
    let (host, port) = (config.host.clone(), config.port);

    info!("initializing gemini-juggler...");

    let shared_juggler = Arc::new(RwLock::new(KeyJuggler::new(config.keys.clone())));

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
            .app_data(web::Data::new(AppState::new(
                config.clone(),
                shared_juggler.clone(),
            )))
            .service(routes::completion)
            .service(routes::openai_completion)
            .service(routes::status)
    })
    .bind((host, port))?
    .run()
    .await
    .map_err(Into::into)
}
