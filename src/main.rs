#![forbid(unsafe_code)]
mod config;
mod generator;

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::Router;
use axum_streams::StreamBodyAs;
use config::Config;
use generator::Generator;
use generator::PandorasGenerator;
use std::path::PathBuf;
use std::process::exit;
use tokio::net::TcpListener;

/// Uses `gen` to stream an infinite text stream.
///
/// Sets some headers, like `Content-Type` automatically.
async fn text_stream<T: Generator + 'static>(gen: T) -> impl IntoResponse {
    // Set some headers to trick le bots
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "text/html; charset=utf-8".parse().unwrap());

    StreamBodyAs::text(gen.to_stream())
        .headers(headers)
        .into_response()
}

#[tokio::main]
async fn main() {
    // Who needs clap
    let args: Vec<String> = std::env::args().collect();

    let config: Config = if args.len() > 1 {
        let pb = PathBuf::from(args[1].clone());
        Config::from_path(&pb).unwrap()
    } else {
        Config::read_from_default_path().unwrap_or_else(|| match Config::default_path() {
            Some(pb) => {
                println!(
                    "No config found at '{}', using a default instead...",
                    pb.to_string_lossy(),
                );
                Config::default()
            }
            None => {
                println!(
                    "Could not find home directory and config, using default config instead..."
                );
                Config::default()
            }
        })
    };

    let gen = PandorasGenerator::default();
    let mut app = Router::new();

    if config.http.catch_all {
        // Since we have no other routes now, all will be passed to the fallback
        app = app.fallback(get(move || text_stream(gen)));
        println!("Catch-All enabled");
    } else if !config.http.routes.is_empty() {
        for route in &config.http.routes {
            let gen = gen.clone();
            app = app.route(route, get(move || text_stream(gen)));
        }
        println!("Listening on routes: {}", config.http.routes.join(", "));
    } else {
        println!("http.catch_all was disabled, but no routes was provided!");
        exit(1);
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.http.port))
        .await
        .unwrap();
    println!("Listening on port {}", config.http.port);

    axum::serve(listener, app).await.unwrap();
}
