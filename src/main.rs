#![forbid(unsafe_code)]
mod config;
mod generators;

use axum::{http::HeaderMap, response::IntoResponse, routing::*, Router};
use axum_streams::StreamBodyAs;
use config::Config;
use generators::{random::RandomGenerator, Generator};
use std::{fs, path::PathBuf, process::exit};
use tokio::net::TcpListener;
use tracing_subscriber::prelude::*;

use crate::generators::{markov::MarkovChainGenerator, GeneratorType};

/// Container for generators, to avoid trait objects.
#[derive(Clone)]
enum GeneratorContainer {
    Random(RandomGenerator),
    MarkovChain(MarkovChainGenerator),
}

/// Uses `gen` to stream an infinite text stream.
///
/// Sets some headers, like `Content-Type` automatically.
async fn text_stream(gen: GeneratorContainer) -> impl IntoResponse {
    // Set some headers to trick le bots
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "text/html; charset=utf-8".parse().unwrap());

    match gen {
        GeneratorContainer::Random(g) => StreamBodyAs::text(g.to_stream())
            .headers(headers)
            .into_response(),
        GeneratorContainer::MarkovChain(g) => StreamBodyAs::text(g.to_stream())
            .headers(headers)
            .into_response(),
    }
}

#[tokio::main]
async fn main() {
    // Who needs clap
    let args: Vec<String> = std::env::args().collect();
    let config: Config = if args.len() > 1 {
        let pb = PathBuf::from(args[1].clone());
        let c = Config::from_path(&pb);
        match c {
            Some(actual) => actual,
            None => {
                println!(
                    "File at '{}' could not be parsed as proper config",
                    pb.to_string_lossy()
                );
                exit(14);
            }
        }
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

    let json_log = match fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(&config.logging.output_path)
    {
        Ok(file) => tracing_subscriber::fmt::layer().json().with_writer(file),
        Err(e) => {
            println!(
                "failed to open log path '{}' due to error:\n\t{}",
                config.logging.output_path, e
            );
            exit(3);
        }
    };

    // Set file logging (or not, if we had no output path)
    let subscriber = subscriber.with(json_log);
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    // Create gen depending on config
    let gen = match config.generator.generator_type {
        GeneratorType::Random => GeneratorContainer::Random(RandomGenerator::default()),
        GeneratorType::MarkovChain(_) => {
            GeneratorContainer::MarkovChain(MarkovChainGenerator::from_config(config.generator))
        }
    };

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

    // Add tracing to as a layer to our app
    let trace_layer = tower_http::trace::TraceLayer::new_for_http()
        .on_request(tower_http::trace::DefaultOnRequest::new().level(tracing::Level::INFO))
        .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::DEBUG))
        .on_eos(tower_http::trace::DefaultOnEos::new().level(tracing::Level::DEBUG))
        .on_failure(tower_http::trace::DefaultOnFailure::new().level(tracing::Level::DEBUG));

    app = app.layer(trace_layer);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.http.port))
        .await
        .unwrap();
    tracing::info!("Listening on port {}", config.http.port);

    axum::serve(listener, app).await.unwrap();
}
