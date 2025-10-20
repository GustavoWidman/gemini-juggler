use std::future::{Ready, ready};
use std::time::Instant;

use actix_web::Error;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use colored::Colorize;
use dur::Duration;
use futures_util::future::LocalBoxFuture;
use log::{error, info};

pub struct HttpLogger;

impl<S, B> Transform<S, ServiceRequest> for HttpLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = HttpLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(HttpLoggerMiddleware { service }))
    }
}

pub struct HttpLoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for HttpLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let peer_addr = req
            .peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let start = Instant::now();
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let elapsed = start.elapsed();
            let status = res.status().as_u16();

            // Format similar to your custom logger
            let status_colored = colorize_status(status);
            let duration = format!("{:.2}", Duration::from(elapsed));

            // Align columns using tabs/spaces
            let path_display = if path.len() > 50 {
                format!("{:>1}...", &path[..47])
            } else {
                path.clone()
            };

            let log_msg = format!(
                " {} {} {:>8} {} {:<15} {} {:<7}  {:<50}",
                status_colored,
                "|".white(),
                duration.white(),
                "|".white(),
                peer_addr.white(),
                "|".white(),
                method.cyan(),
                path_display.white(),
            );

            match status {
                500..=599 => error!("{}", log_msg),
                _ => info!("{}", log_msg),
            }

            Ok(res)
        })
    }
}

fn colorize_status(status: u16) -> String {
    let status_str = status.to_string();
    match status {
        200..=299 => status_str.green().to_string(),
        300..=399 => status_str.yellow().to_string(),
        400..=499 => status_str.bright_red().to_string(),
        500..=599 => status_str.red().bold().to_string(),
        _ => status_str.white().to_string(),
    }
}
