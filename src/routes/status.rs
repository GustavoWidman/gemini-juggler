use actix_web::{HttpRequest, HttpResponse, get, web};
use serde_json::json;

use crate::AppState;

#[get("/status")]
async fn status(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let auth_header = req.headers().get("Authorization");
    let is_authenticated = match auth_header {
        Some(header) => match header.to_str() {
            Ok(value) => {
                if let Some(token) = value.strip_prefix("Bearer ") {
                    token == data.config.api_key
                } else {
                    false
                }
            }
            Err(_) => false,
        },
        None => false,
    };

    if !is_authenticated {
        return HttpResponse::Unauthorized()
            .json(json!({"error": "missing or invalid authorization header"}));
    }

    let mut juggler = data.juggler.write().await;
    let statuses = juggler.get_status();

    HttpResponse::Ok().json(json!({
        "keys": statuses,
        "total_keys": statuses.len(),
        "active_keys": statuses.iter().filter(|s| !s.is_ratelimited).count(),
        "ratelimited_keys": statuses.iter().filter(|s| s.is_ratelimited).count(),
    }))
}
