use axum::{body::Body, http::Request};
use tracing::Span;

/// Struct used to describe to tower trace middleware what to print.
///
/// Assumes to be behind a reverse proxy, so attempts to print IP from
/// common headers set by reverse proxies.
#[derive(Debug, Clone)]
pub(crate) struct RequestHandler;

impl RequestHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl tower_http::trace::OnRequest<Body> for RequestHandler {
    fn on_request(&mut self, request: &Request<Body>, current_span: &Span) {
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

        let proxied_ip = client_ip.map_or("unknown", |ip| ip.to_str().unwrap_or("unknown"));

        current_span.record("proxied_ip", proxied_ip);
        tracing::info!(
            "Hostile proxied IP '{}' connected to URI '{}'",
            proxied_ip,
            request.uri()
        );
    }
}
