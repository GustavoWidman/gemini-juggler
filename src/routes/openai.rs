use actix_web::{Error, HttpRequest, HttpResponse, post, web};
use futures_util::TryStreamExt;
use log::error;
use serde_json::{Value, json};

use crate::{AppState, utils::Event};

fn extract_bearer_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

#[post("/v1beta/openai/chat/completions")]
async fn openai_completion(
    req: HttpRequest,
    body: web::Json<Value>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let key = extract_bearer_token(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Missing or invalid Authorization header")
    })?;

    let data = data.into_inner();

    if key != data.config.api_key {
        return Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid API key"})));
    }

    let body = body.into_inner();
    let is_streaming = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut juggler = data.juggler.write().await;

    loop {
        let key = juggler.current().to_string();
        match data.requester.forward_openai(&key, &body).await? {
            Event::Ok(resp) => return Ok(resp),
            Event::Forward(mut resp) => match is_streaming {
                true => {
                    let stream = resp.into_stream();
                    let mut response = HttpResponse::Ok();
                    return Ok(response.streaming(stream));
                }
                false => {
                    let body_bytes = resp.body().await.map_err(|e| {
                        actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e))
                    })?;
                    return Ok(HttpResponse::build(resp.status()).body(body_bytes));
                }
            },
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
