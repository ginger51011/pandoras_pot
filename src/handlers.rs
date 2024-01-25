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
        let headers = request.headers();

        // We try to find the IP, we are probably behind a reverse proxy, so try common ones.
        // It's ok if this takes a little time (compiled Rust wont), since the real fun begins
        // later
        let mut client_ip = None;
        for header_name in [
            "CF-Connecting-IP",
            "X-Forwarded-For",
            "X-Real-IP",
            "Client-IP",
            "X-Originating-IP",
            "Forwarded",
        ] {
            if let Some(value) = headers.get(header_name) {
                client_ip = Some(value);
                break;
            }
        }

        let client_ip = client_ip
            .map(|ip| ip.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown");

        tracing::info!(
            "Hostile IP '{}' connected to URI '{}'",
            client_ip,
            request.uri()
        );
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
