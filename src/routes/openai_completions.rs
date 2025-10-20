use actix_web::{Error, HttpRequest, HttpResponse, post, web};
use awc::http::StatusCode;
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
    let key = extract_bearer_token(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Missing or invalid Authorization header")
    })?;

    let data = data.into_inner();

    if key != data.config.api_key {
        return Ok(HttpResponse::Unauthorized().json(json!({
            "error": "Invalid API key"
        })));
    }

    let body = body.into_inner();

    // Check if streaming is requested
    let is_streaming = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut juggler = data.juggler.write().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Error locking juggler: {}", e))
    })?;
    let mut gemini_api_key = juggler.current();

    if is_streaming {
        // Streaming mode
        let gemini_response = loop {
            log::debug!("Forwarding streaming OpenAI-compatible request to Gemini, using key {gemini_api_key}");
            let mut gemini_response = data
                .client
                .post("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions")
                .insert_header(("Authorization", format!("Bearer {}", gemini_api_key)))
                .insert_header(("Content-Type", "application/json"))
                .no_decompress()
                .send_json(&body)
                .await
                .map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error forwarding request: {}", e))
                })?;

            let status_code = gemini_response.status();

            if status_code == StatusCode::TOO_MANY_REQUESTS {
                let body_bytes = gemini_response.body().await.map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e))
                })?;

                if String::from_utf8_lossy(&body_bytes)
                    .to_lowercase()
                    .contains("day")
                {
                    gemini_api_key =
                        juggler
                            .ratelimit()
                            .ok_or(actix_web::error::ErrorTooManyRequests(
                                "All API keys are ratelimited"
                            ))?;

                    continue;
                } else {
                    // not a daily ratelimit error, return the response as usual (boil up the error)
                    return Ok(HttpResponse::build(status_code).body(body_bytes));
                }
            } else if !status_code.is_success() {
                // For other error status codes, return the response immediately
                let body_bytes = gemini_response.body().await.map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e))
                })?;
                return Ok(HttpResponse::build(status_code).body(body_bytes));
            } else {
                break gemini_response;
            }
        };

        // Stream the response
        let mut response = HttpResponse::Ok();
        for (key, value) in gemini_response.headers().iter() {
            response.insert_header((key.to_string(), value.to_str().unwrap_or("").to_string()));
        }

        let stream = gemini_response.into_stream();
        Ok(response.streaming(stream))
    } else {
        // Non-streaming mode (original behavior)
        let (gemini_response, response_body, status_code) = loop {
            log::debug!("Forwarding OpenAI-compatible request to Gemini, using key {gemini_api_key}");
            let mut gemini_response = data
                .client
                .post("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions")
                .insert_header(("Authorization", format!("Bearer {}", gemini_api_key)))
                .insert_header(("Content-Type", "application/json"))
                .no_decompress()
                .send_json(&body)
                .await
                .map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error forwarding request: {}", e))
                })?;

            let status_code = gemini_response.status();

            if status_code == StatusCode::TOO_MANY_REQUESTS {
                let body = gemini_response.body().await.map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e))
                })?;

                if String::from_utf8_lossy(&body)
                    .to_lowercase()
                    .contains("day")
                {
                    gemini_api_key =
                        juggler
                            .ratelimit()
                            .ok_or(actix_web::error::ErrorTooManyRequests(
                                "All API keys are ratelimited"
                            ))?;

                    continue;
                } else {
                    // not a daily ratelimit error, return the response as usual (boil up the error)
                    break Ok::<_, actix_web::Error>((gemini_response, body, status_code));
                }
            } else {
                let response_body = gemini_response.body().await.map_err(|e| {
                    actix_web::error::ErrorBadGateway(format!("Error reading response: {}", e))
                })?;

                break Ok::<_, actix_web::Error>((gemini_response, response_body, status_code));
            }
        }?;

        let mut response = HttpResponse::build(status_code);

        for (key, value) in gemini_response.headers().iter() {
            response.insert_header((key.to_string(), value.to_str().unwrap().to_string()));
        }

        let response = response.body(response_body);

        Ok(response)
    }
}
