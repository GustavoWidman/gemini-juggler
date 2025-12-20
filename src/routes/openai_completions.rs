use actix_web::{Error, HttpRequest, HttpResponse, post, web};
use awc::http::StatusCode;
use colored::Colorize;
use futures_util::TryStreamExt;
use serde_json::{Value, json};

use crate::AppState;

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
    let key = extract_bearer_token(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing or invalid Authorization header"))?;

    let data = data.into_inner();

    if key != data.config.api_key {
        return Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid API key"})));
    }

    let body = body.into_inner();
    let is_streaming = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut juggler = data.juggler.write().await;

    if is_streaming {
        handle_streaming(&data, &mut juggler, &body).await
    } else {
        handle_non_streaming(&data, &mut juggler, &body).await
    }
}

async fn handle_streaming(
    data: &AppState,
    juggler: &mut crate::utils::KeyJuggler,
    body: &Value,
) -> Result<HttpResponse, Error> {
    loop {
        let key = juggler.current();
        log::debug!("forwarding streaming openai-compatible request to {}, using key {}", "gemini".cyan(), key.to_string().cyan());

        let mut resp = data
            .client
            .post("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions")
            .insert_header(("Authorization", format!("Bearer {}", key)))
            .insert_header(("Content-Type", "application/json"))
            .no_decompress()
            .send_json(body)
            .await
            .map_err(|e| actix_web::error::ErrorBadGateway(format!("Error forwarding request: {}", e)))?;

        match resp.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                let body_bytes = resp
                    .body()
                    .await
                    .map_err(|e| actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e)))?;

                if String::from_utf8_lossy(&body_bytes).to_lowercase().contains("day") {
                    juggler.ratelimit().ok_or(actix_web::error::ErrorTooManyRequests("All API keys are ratelimited"))?;
                    continue;
                }
                return Ok(HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).body(body_bytes));
            }
            status if status.is_success() => {
                let stream = resp.into_stream();
                let mut response = HttpResponse::Ok();
                return Ok(response.streaming(stream));
            }
            status => {
                let body_bytes = resp
                    .body()
                    .await
                    .map_err(|e| actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e)))?;
                return Ok(HttpResponse::build(status).body(body_bytes));
            }
        }
    }
}

async fn handle_non_streaming(
    data: &AppState,
    juggler: &mut crate::utils::KeyJuggler,
    body: &Value,
) -> Result<HttpResponse, Error> {
    loop {
        let key = juggler.current();
        log::debug!("forwarding openai-compatible request to {}, using key {}", "gemini".cyan(), key.to_string().cyan());

        let mut resp = data
            .client
            .post("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions")
            .insert_header(("Authorization", format!("Bearer {}", key)))
            .insert_header(("Content-Type", "application/json"))
            .no_decompress()
            .send_json(body)
            .await
            .map_err(|e| actix_web::error::ErrorBadGateway(format!("Error forwarding request: {}", e)))?;

        let status = resp.status();

        match status {
            StatusCode::TOO_MANY_REQUESTS => {
                let body_bytes = resp
                    .body()
                    .await
                    .map_err(|e| actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e)))?;

                if String::from_utf8_lossy(&body_bytes).to_lowercase().contains("day") {
                    juggler.ratelimit().ok_or(actix_web::error::ErrorTooManyRequests("All API keys are ratelimited"))?;
                    continue;
                }
                return Ok(HttpResponse::build(status).body(body_bytes));
            }
            _ => {
                let body_bytes = resp
                    .body()
                    .await
                    .map_err(|e| actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e)))?;
                return Ok(HttpResponse::build(status).body(body_bytes));
            }
        }
    }
}
