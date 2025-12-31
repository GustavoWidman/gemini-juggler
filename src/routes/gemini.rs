use actix_web::{Error, HttpResponse, post, web};
use futures_util::TryStreamExt;
use log::error;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, utils::Event};

#[derive(Deserialize)]
struct Query {
    key: String,
}

#[post("/v1beta/models/{model}:generateContent")]
async fn completion(
    path: web::Path<String>,
    query: web::Query<Query>,
    body: web::Json<Value>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let key = query.into_inner().key;
    let data = data.into_inner();

    if key != data.config.api_key {
        return Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid API key"})));
    }

    let model = path.into_inner();
    let body = body.into_inner();
    let mut juggler = data.juggler.write().await;

    loop {
        let key = juggler.current().to_string();
        match data
            .requester
            .forward_gemini(&key, &model, &body, false)
            .await?
        {
            Event::Ok(resp) => return Ok(resp),
            Event::Forward(mut resp) => {
                let body_bytes = resp.body().await.map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e))
                })?;
                return Ok(HttpResponse::build(resp.status()).body(body_bytes));
            }
            Event::Fail(e) => return Err(e),
            Event::Retry => {
                juggler
                    .ratelimit(&key)
                    .ok_or(actix_web::error::ErrorTooManyRequests(
                        "All API keys are ratelimited",
                    ))?;
                continue;
            }
            Event::BadKey => {
                error!("received indication of bad key, removing from rotation and retrying...");
                juggler.remove(&key);
                continue;
            }
        }
    }
}

#[post("/v1beta/models/{model}:streamGenerateContent")]
async fn stream_completion(
    path: web::Path<String>,
    query: web::Query<Query>,
    body: web::Json<Value>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let key = query.into_inner().key;
    let data = data.into_inner();

    if key != data.config.api_key {
        return Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid API key"})));
    }

    let model = path.into_inner();
    let body = body.into_inner();
    let mut juggler = data.juggler.write().await;

    loop {
        let key = juggler.current().to_string();
        match data
            .requester
            .forward_gemini(&key, &model, &body, false)
            .await?
        {
            Event::Ok(resp) => return Ok(resp),
            Event::Forward(resp) => {
                let stream = resp.into_stream();
                let mut response = HttpResponse::Ok();
                return Ok(response.streaming(stream));
            }
            Event::Fail(e) => return Err(e),
            Event::Retry => {
                juggler
                    .ratelimit(&key)
                    .ok_or(actix_web::error::ErrorTooManyRequests(
                        "All API keys are ratelimited",
                    ))?;
                continue;
            }
            Event::BadKey => {
                error!("received indication of bad key, removing from rotation and retrying...");
                juggler.remove(&key);
                continue;
            }
        }
    }
}
