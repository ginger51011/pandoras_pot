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
use std::path::PathBuf;
use std::process::exit;

async fn text_stream(gen: PandorasGenerator) -> impl IntoResponse {
    StreamBodyAs::text(gen.to_stream())
}

#[tokio::main]
async fn main() {
    // Who needs clap
    let args: Vec<String> = std::env::args().collect();

    let config: Config;

    if args.len() > 1 {
        let pb = PathBuf::from(args[1].clone());
        config = Config::from_path(&pb).unwrap();
    } else {
        config = Config::read_from_default_path().unwrap_or_else(|| match Config::default_path() {
            Some(pb) => {
                println!(
                    "No default config found at '{}', using a default instead...",
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
        });
    }

    let gen = PandorasGenerator::default();
    let mut app = Router::new();

    if config.http.catch_all {
        // Since we have no other routes now, all will be passed to the fallback
        app = app.fallback(get(move || text_stream(gen)));
    } else if !config.http.routes.is_empty() {
        for route in config.http.routes {
            let gen = gen.clone();
            app = app.route(&route, get(move || text_stream(gen)));
        }
    } else {
        println!("http.catch_all was disabled, but no routes was provided!");
        exit(1);
    }

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
