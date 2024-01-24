#![forbid(unsafe_code)]
use axum::response::IntoResponse;
use axum::routing::*;
use axum::Router;
use axum_streams::StreamBodyAs;
use config::Config;
use generator::PandorasGenerator;
use tokio::net::TcpListener;
mod config;
mod generator;

async fn text_stream(gen: PandorasGenerator) -> impl IntoResponse {
    StreamBodyAs::text(gen.to_stream())
}

#[tokio::main]
async fn main() {
    let config = Config::default();
    let gen = PandorasGenerator::default();
    let mut app = Router::new();

    if config.http.catch_all {
        // Since we have no other routes now, all will be passed to the fallback
        app = app.fallback(get(move || text_stream(gen)));
    } else {
        for route in config.http.routes {
            let gen = gen.clone();
            app = app.route(&route, get(move || text_stream(gen)));
        }
    }

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
