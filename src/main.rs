#![forbid(unsafe_code)]
mod config;
mod error_code;
mod generator;
mod handler;

use axum::{
    error_handling::HandleErrorLayer,
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::*,
    BoxError, Router,
};
use axum_streams::StreamBodyAs;
use config::Config;
use generator::{random_generator::RandomGenerator, Generator, GeneratorContainer};
use std::{fs, path::PathBuf, process::exit, time::Duration};
use tokio::net::TcpListener;
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tracing_subscriber::prelude::*;

use crate::{
    config::GeneratorType,
    generator::{markov_generator::MarkovChainGenerator, static_generator::StaticGenerator},
    handler::RequestHandler,
};

async fn text_stream(gen: GeneratorContainer) -> impl IntoResponse {
    // Set some headers to trick le bots
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());

    match gen {
        GeneratorContainer::Random(g) => StreamBodyAs::text(g.into_stream()).headers(headers),
        GeneratorContainer::MarkovChain(g) => StreamBodyAs::text(g.into_stream()).headers(headers),
        GeneratorContainer::Static(g) => StreamBodyAs::text(g.into_stream()).headers(headers),
    }
}

/// Creates a new app from a config.
fn create_app(config: &Config) -> Router {
    let mut app = Router::new();

    // Create gen depending on config
    tracing::info!("Using generator: {}", config.generator.generator_type);
    let gen = match config.generator.generator_type {
        GeneratorType::Random => {
            GeneratorContainer::Random(RandomGenerator::from_config(config.generator.to_owned()))
        }
        GeneratorType::MarkovChain(_) => GeneratorContainer::MarkovChain(
            MarkovChainGenerator::from_config(config.generator.to_owned()),
        ),
        GeneratorType::Static(_) => {
            GeneratorContainer::Static(StaticGenerator::from_config(config.generator.to_owned()))
        }
    };

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
        exit(error_code::BAD_CONFIG);
    }

    // Add tracing to as a layer to our app
    let trace_layer = tower_http::trace::TraceLayer::new_for_http()
        .on_request(RequestHandler::new())
        .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::DEBUG))
        .on_eos(tower_http::trace::DefaultOnEos::new().level(tracing::Level::DEBUG))
        .on_failure(tower_http::trace::DefaultOnFailure::new().level(tracing::Level::DEBUG));

    app = app.layer(trace_layer);

    // Set rate limiting

    // u64, so not below zero
    if config.http.rate_limit != 0 {
        if config.http.rate_limit_period == 0 {
            println!("You cannot activate rate limiting and then set the period to 0!");
            exit(error_code::BAD_CONFIG);
        }
        // See https://github.com/tokio-rs/axum/discussions/987#discussioncomment-2678115
        app = app.layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err: BoxError| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled error: {}", err),
                    )
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(
                    config.http.rate_limit,
                    Duration::from_secs(config.http.rate_limit_period),
                )),
        );
    };

    app
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
                exit(error_code::UNPARSEABLE_CONFIG);
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
            exit(error_code::CANNOT_OPEN_LOG_FILE);
        }
    };

    // Set file logging (or not, if we had no output path)
    let subscriber = subscriber.with(json_log);
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    let app = create_app(&config);

    if config.http.health_port_enabled {
        if config.http.port == config.http.health_port {
            println!(
                "Health port and normal port cannot be the same! (Both are {})",
                config.http.port
            );
            exit(error_code::BAD_CONFIG);
        }

        // Use fallback to always respond with the same value
        let health_router = Router::new().fallback_service(get(|| async { "OK\n" }));
        let health_listener = TcpListener::bind(format!("0.0.0.0:{}", config.http.health_port))
            .await
            .unwrap();
        tracing::info!("Health check listening on port {}", config.http.health_port);
        tokio::spawn(async move { axum::serve(health_listener, health_router).await.unwrap() });
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.http.port))
        .await
        .unwrap();
    tracing::info!("Listening on port {}", config.http.port);

    axum::serve(listener, app).await.unwrap();
}
