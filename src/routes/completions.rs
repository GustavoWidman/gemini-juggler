use actix_web::{Error, HttpResponse, post, web};
use awc::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;

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
        return Ok(HttpResponse::Unauthorized().json(json!({
            "error": "Invalid API key"
        })));
    }

    let model = path.into_inner();
    let body = body.into_inner();

    let mut juggler = data.juggler.write().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Error locking juggler: {}", e))
    })?;
    let mut gemini_api_key = juggler.current();
    let (gemini_response, response_body, status_code) = loop {
        log::debug!("Forwarding request to Gemini, using key {gemini_api_key}");
        let mut gemini_response = data
			.client
			.post(format!(
				"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={gemini_api_key}"
			))
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
                        .ok_or(actix_web::error::ErrorTooManyRequests(format!(
                            "All API keys are ratelimited"
                        )))?;

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
