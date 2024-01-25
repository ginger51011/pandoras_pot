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
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use tokio::net::TcpListener;
use tracing;
use tracing_subscriber::prelude::*;

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

    // Set up tracing
    let (pretty, ugly) = if config.logging.no_stdout {
        (None, None)
    } else if config.logging.print_pretty_logs {
        (Some(tracing_subscriber::fmt::layer().pretty()), None)
    } else {
        (None, Some(tracing_subscriber::fmt::layer()))
    };

    // None will be ignored, so we will in reality only have one
    let subscriber = tracing_subscriber::Registry::default()
        .with(pretty)
        .with(ugly);

    let json_log = config.logging.output_path.and_then(|output_path| {
        match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(&output_path)
        {
            Ok(file) => {
                let json_log = tracing_subscriber::fmt::layer().json().with_writer(file);
                Some(json_log)
            }
            Err(e) => {
                println!(
                    "failed to open log path '{}' due to error:\n\t{}",
                    output_path, e
                );
                exit(3);
            }
        }
    });

    // Set file logging (or not, if we had no output path)
    let subscriber = subscriber.with(json_log);
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    let gen = PandorasGenerator::default();
    let mut app = Router::new();

    if config.http.catch_all {
        // Since we have no other routes now, all will be passed to the fallback
        app = app.fallback(get(move || text_stream(gen)));
        tracing::info!("Catch-All enabled");
    } else if !config.http.routes.is_empty() {
        for route in &config.http.routes {
            let gen = gen.clone();
            app = app.route(route, get(move || text_stream(gen)));
        }
        tracing::info!("Listening on routes: {}", config.http.routes.join(", "));
    } else {
        tracing::info!("http.catch_all was disabled, but no routes was provided!");
        exit(1);
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.http.port))
        .await
        .unwrap();
    tracing::info!("Listening on port {}", config.http.port);

    axum::serve(listener, app).await.unwrap();
}
