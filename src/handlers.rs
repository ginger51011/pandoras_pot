use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use std::time::Duration;
use tracing::Span;

/// Struct used to describe to tower trace middleware what to print.
#[derive(Debug, Clone)]
pub(crate) struct RequestHandler;

impl RequestHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl tower_http::trace::OnRequest<Body> for RequestHandler {
    fn on_request(&mut self, request: &Request<Body>, _: &Span) {
        let client_ip = request
            .headers()
            .get("X-Forwarded-For") // TODO: Less stupid solution
            .map(|ip| ip.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown");

        tracing::info!("Hostile IP '{}' connected", client_ip,);
    }
}

/// Struct used to describe to tower trace middleware what to print.
#[derive(Debug, Clone)]
pub(crate) struct EosHandler;

impl EosHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl tower_http::trace::OnEos for EosHandler {
    fn on_eos(self, trailer: Option<&HeaderMap>, stream_duration: Duration, _: &Span) {
        let client_ip = match trailer {
            Some(h) => h.get("X-Forwarded-For"),
            None => None,
        };

        let client_ip = client_ip
            .map(|ip| ip.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown");

        tracing::info!(
            "Ended connection for IP '{}' disconnected after {} seconds ({} minutes)",
            client_ip,
            stream_duration.as_secs(),
            stream_duration.as_secs() / 60,
        );
    }
}
