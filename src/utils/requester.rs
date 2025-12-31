use std::pin::Pin;

use actix_web::{Error, HttpResponse, dev::Decompress, error::ErrorBadGateway};
use awc::{Client, ClientResponse, error::PayloadError, http::StatusCode};
use colored::Colorize;
use log::error;
use serde_json::Value;

type Response = ClientResponse<
    Decompress<
        actix_web::dev::Payload<
            Pin<Box<dyn futures_util::Stream<Item = Result<actix_web::web::Bytes, PayloadError>>>>,
        >,
    >,
>;
pub enum Event {
    Ok(HttpResponse),
    Forward(Response),
    Retry,
    BadKey, // maybe rename this
    Fail(Error),
}

pub struct Requester {
    client: Client,
}

impl Requester {
    pub fn new() -> Self {
        Self {
            client: Client::builder().disable_timeout().finish(),
        }
    }

    pub async fn forward_gemini(
        &self,
        key: &str,
        model: &str,
        body: &Value,
        stream: bool,
    ) -> Result<Event, Error> {
        let url = match stream {
            true => format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?key={}&alt=sse",
                key
            ),
            false => format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={}",
                key
            ),
        };

        log::debug!(
            "forwarding request to {}, using key {}",
            "gemini".cyan(),
            key.cyan()
        );

        let resp = self
            .client
            .post(&url)
            .no_decompress()
            .send_json(body)
            .await
            .map_err(|e| {
                actix_web::error::ErrorBadGateway(format!("Error forwarding request: {}", e))
            })?;

        Ok(Self::handle_status(resp).await)
    }

    pub async fn forward_openai(&self, key: &str, body: &Value) -> Result<Event, Error> {
        let resp = self
            .client
            .post("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions")
            .insert_header(("Authorization", format!("Bearer {}", key)))
            .insert_header(("Content-Type", "application/json"))
            .no_decompress()
            .send_json(body)
            .await
            .map_err(|e| {
                actix_web::error::ErrorBadGateway(format!("Error forwarding request: {}", e))
            })?;

        Ok(Self::handle_status(resp).await)
    }

    async fn handle_status(mut resp: Response) -> Event {
        match resp.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                let body_bytes = match resp.body().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        return Event::Fail(ErrorBadGateway(format!(
                            "Error reading response: {}",
                            e
                        )));
                    }
                };

                if String::from_utf8_lossy(&body_bytes)
                    .to_lowercase()
                    .contains("day")
                {
                    return Event::Retry; // ratelimited
                }

                if body_bytes.len() == 344 {
                    error!(
                        "received 344 byte body, indicating a broken key, removing from rotation and retrying..."
                    );

                    return Event::BadKey;
                }

                Event::Ok(HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).body(body_bytes))
            }
            status if status.is_success() => Event::Forward(resp),
            status => {
                let body_bytes = match resp.body().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        return Event::Fail(ErrorBadGateway(format!(
                            "Error reading response: {}",
                            e
                        )));
                    }
                };
                Event::Ok(HttpResponse::build(status).body(body_bytes))
            }
        }
    }
}
