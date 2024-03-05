#![forbid(unsafe_code)]
mod config;
mod error_code;
mod generator;
mod handler;
mod stream_body;

use axum::{
    error_handling::HandleErrorLayer,
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, on, MethodFilter},
    BoxError, Router,
};
use config::Config;
use generator::{random_generator::RandomGenerator, Generator, GeneratorContainer};
use std::{fs, path::PathBuf, process::exit, time::Duration};
use stream_body::StreamBody;
use tokio::net::TcpListener;
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tower_http::trace::MakeSpan;
use tracing::info_span;
use tracing_subscriber::prelude::*;

use crate::{
    config::GeneratorType,
    generator::{
        markov_generator::MarkovChainGenerator, static_generator::StaticGenerator, P_TAG_SIZE,
    },
    handler::RequestHandler,
};

/// Used to provide a span that will hold metadata about a connection, so it can be tracked.
#[derive(Clone, Debug)]
struct PandoraRequestSpan;
impl<B> MakeSpan<B> for PandoraRequestSpan {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> tracing::Span {
        let version_string = match request.version() {
            axum::http::Version::HTTP_09 => "HTTP/0.9".to_string(),
            axum::http::Version::HTTP_10 => "HTTP/1.0".to_string(),
            axum::http::Version::HTTP_11 => "HTTP/1.1".to_string(),
            axum::http::Version::HTTP_2 => "HTTP/2".to_string(),
            axum::http::Version::HTTP_3 => "HTTP/3".to_string(),
            _ => "UNKNOWN".to_string(),
        };
        info_span!(
            "request",
            version = version_string,
            method = request.method().to_string(),
            uri = request.uri().to_string(),
            proxied_ip = tracing::field::Empty, // Set later by our RequestHandler
            origin_ip = tracing::field::Empty,  // TODO: Same as above, will generally be the
                                                // reverse proxy
        )
    }
}

async fn text_stream(gen: GeneratorContainer) -> impl IntoResponse {
    // Set some headers to trick le bots
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());

    match gen {
        GeneratorContainer::Random(g) => StreamBody::from_stream(g.into_stream()).headers(headers),
        GeneratorContainer::MarkovChain(g) => {
            StreamBody::from_stream(g.into_stream()).headers(headers)
        }
        GeneratorContainer::Static(g) => StreamBody::from_stream(g.into_stream()).headers(headers),
    }
}

/// Creates a new app from a config.
///
/// Returns an exit code in case of configuration errors.
fn create_app(config: &Config) -> Result<Router, i32> {
    // This will mess upp for example markov
    if config.generator.chunk_size < P_TAG_SIZE {
        println!(
            "generator.chunk_size too small (min size is {}, but it should be bigger!)",
            P_TAG_SIZE
        );
        return Err(error_code::GENERATOR_CHUNK_SIZE_TOO_SMALL);
    }

    let mut app = Router::new();

    // Create gen depending on config
    tracing::info!("Using generator: {}", config.generator.generator_type);
    let gen = match config.generator.generator_type {
        GeneratorType::Random => {
            GeneratorContainer::Random(RandomGenerator::from_config(config.generator.clone()))
        }
        GeneratorType::MarkovChain(_) => GeneratorContainer::MarkovChain(
            MarkovChainGenerator::from_config(config.generator.clone()),
        ),
        GeneratorType::Static(_) => {
            GeneratorContainer::Static(StaticGenerator::from_config(config.generator.clone()))
        }
    };

    const ANY_METHOD: MethodFilter = MethodFilter::DELETE
        .or(MethodFilter::GET)
        .or(MethodFilter::HEAD) // TODO: Acutally transmit an infinite header
        .or(MethodFilter::OPTIONS)
        .or(MethodFilter::PATCH)
        .or(MethodFilter::POST)
        .or(MethodFilter::PUT)
        .or(MethodFilter::TRACE);

    let handler = move || text_stream(gen);
    if config.http.catch_all {
        // Since we have no other routes now, all will be passed to the fallback
        app = app.fallback(on(ANY_METHOD, handler));
        tracing::info!("Catch-All enabled");
    } else if !config.http.routes.is_empty() {
        for route in &config.http.routes {
            let handler = handler.clone();
            app = app.route(route, on(ANY_METHOD, handler));
        }
        tracing::info!("Listening on routes: {}", config.http.routes.join(", "));
    } else {
        println!("http.catch_all was disabled, but no routes was provided!");
        return Err(error_code::BAD_CONFIG);
    }

    // Add tracing to as a layer to our app, span must hold some records that we are interested in
    let trace_layer = tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(PandoraRequestSpan)
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
            return Err(error_code::BAD_CONFIG);
        }
        // See https://github.com/tokio-rs/axum/discussions/987#discussioncomment-2678115
        app = app.layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err: BoxError| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled error: {err}"),
                    )
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(
                    config.http.rate_limit,
                    Duration::from_secs(config.http.rate_limit_period),
                )),
        );
    };

    Ok(app)
}

#[tokio::main]
async fn main() {
    // Who needs clap
    let args: Vec<String> = std::env::args().collect();
    let config: Config = if args.len() > 1 {
        let pb = PathBuf::from(args[1].clone());
        let c = Config::from_path(&pb);
        if let Some(actual) = c {
            actual
        } else {
            println!(
                "File at '{}' could not be parsed as proper config",
                pb.to_string_lossy()
            );
            exit(error_code::UNPARSEABLE_CONFIG);
        }
    } else {
        Config::read_from_default_path().unwrap_or_else(|| {
            if let Some(pb) = Config::default_path() {
                println!(
                    "No config found at '{}', using a default instead...",
                    pb.to_string_lossy(),
                );
                Config::default()
            } else {
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
    tracing::info!(
        "Running {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let app = match create_app(&config) {
        Ok(a) => a,
        Err(code) => exit(code),
    };

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

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        time::{self, Duration},
    };

    use axum::{
        body::Body,
        extract::Request,
        http::{Method, StatusCode},
        Router,
    };
    use tempfile::NamedTempFile;
    use tokio_stream::StreamExt;
    use tower::ServiceExt; // `oneshot`

    use crate::{
        config::{Config, GeneratorType},
        create_app, error_code,
        generator::{FIRST_MSG_PREFIX, P_TAG_SIZE},
    };

    /// Tests if an app responds with what seems like an infinite stream on
    /// an URI.
    async fn app_responds_on_uri(app: Router, uri: &str) -> bool {
        for method in &[Method::GET, Method::POST, Method::DELETE] {
            let app = app.clone();
            let response = app
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            if response.status() != StatusCode::OK {
                return false;
            }

            // We're safe until we try to actually consume the body. But we can
            // check if it _looks_ like an infinite stream.
            let mut body = response.into_body().into_data_stream();
            for _ in 0..1000 {
                match body.next().await {
                    Some(b) => assert!(b.unwrap().len() > 0),
                    None => return false,
                };
            }
        }
        true
    }

    #[tokio::test]
    async fn app_default_config() {
        let config = Config::default();
        let app = create_app(&config).unwrap();
        assert!(
            app_responds_on_uri(app, "/").await,
            "app did not respond on root uri"
        );
    }

    #[tokio::test]
    async fn app_too_small_chunk_size() {
        let mut config = Config::default();
        config.generator.chunk_size = P_TAG_SIZE - 3;
        match create_app(&config) {
            Err(code) => assert_eq!(code, error_code::GENERATOR_CHUNK_SIZE_TOO_SMALL),
            _ => panic!("too small chunk size was allowed"),
        }
    }

    #[tokio::test]
    async fn app_catch_all() {
        let mut config = Config::default();
        // Just to be sure
        config.http.catch_all = true;

        // These can be set but should have no effect
        config.http.routes = vec!["/wp-login.php".to_string(), "/.git/config".to_string()];

        let app = create_app(&config).unwrap();

        let mut test_routes = vec!["/".to_string(), "/.git".to_string(), "k".to_string()];
        test_routes.append(&mut config.http.routes);

        // But it should on these
        for uri in test_routes.iter() {
            assert!(
                app_responds_on_uri(app.to_owned(), uri).await,
                "app did not respond on {} but it should",
                uri
            )
        }
    }

    #[tokio::test]
    async fn app_specified_routes() {
        let mut config = Config::default();
        config.http.catch_all = false;
        config.http.routes = vec!["/wp-login.php".to_string(), "/.git/config".to_string()];

        let app = create_app(&config).unwrap();

        // It should not respond on these
        for uri in ["/", ".git", "/home"] {
            assert!(
                !app_responds_on_uri(app.to_owned(), uri).await,
                "app did respond on {} but it should not have",
                uri
            )
        }

        // But it should on these
        for uri in &config.http.routes {
            assert!(
                app_responds_on_uri(app.to_owned(), uri).await,
                "app did not respond on {} but it should",
                uri
            )
        }
    }

    #[tokio::test]
    async fn app_with_static_generator() {
        let msg = "I'm the real slim shady".to_string();
        let mut tmpfile: NamedTempFile = tempfile::NamedTempFile::new().unwrap();
        let _ = tmpfile.write(msg.as_bytes()).unwrap();

        let mut config = Config::default();
        config.generator.generator_type = GeneratorType::Static(tmpfile.path().to_path_buf());

        let app = create_app(&config).unwrap();

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // We're safe until we try to actually consume the body. But we can
        // check if it _looks_ like an infinite stream.
        let mut body = response.into_body().into_data_stream();

        // First one should contain tags as well
        let first = body.next().await.unwrap().unwrap();
        assert_eq!(first, format!("{FIRST_MSG_PREFIX}{msg}"));

        // All the following should be our very useful message
        for _ in 0..1000 {
            let chunk = body.next().await.unwrap().unwrap();
            assert_eq!(chunk, msg);
        }
    }

    #[test]
    fn app_disabled_catch_all_no_routes() {
        let mut config = Config::default();
        config.http.catch_all = false;
        config.http.routes = vec![];
        match create_app(&config) {
            Ok(_) => {
                panic!("app created although catch all was disabled but no routes were provided")
            }
            Err(code) => assert_eq!(
                code,
                error_code::BAD_CONFIG,
                "expected error code {} for BAD_CONFIG but got {}",
                error_code::BAD_CONFIG,
                code
            ),
        }
    }

    #[tokio::test]
    async fn app_size_limited() {
        let mut config = Config::default();
        config.generator.size_limit = 1;

        let app = create_app(&config).unwrap();

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // We're safe until we try to actually consume the body. But we can
        // check if it _looks_ like an infinite stream.
        let mut body = response.into_body().into_data_stream();

        // First should be fine, it is never limited
        let first = body.next().await.unwrap().unwrap();
        assert!(first.len() > 0);

        // The next one should be over the limit and the stream should
        // have closed
        match body.next().await {
            Some(_) => panic!("Size limited app sent too much data"),
            None => return,
        }
    }

    #[tokio::test]
    async fn app_time_limited() {
        let mut config = Config::default();
        config.generator.time_limit = 1;

        let app = create_app(&config).unwrap();

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let start_time = time::SystemTime::now();

        // We're safe until we try to actually consume the body. But we can
        // check if it _looks_ like an infinite stream.
        let mut body = response.into_body().into_data_stream();

        // First should be fine, it is never limited
        let first = body.next().await.unwrap().unwrap();
        assert!(first.len() > 0);

        // Take for a while
        while Duration::from_millis(1010) > start_time.elapsed().unwrap() {
            let _ = body.next().await;
        }

        // The next one should be over the limit and the stream should
        // have closed
        match body.next().await {
            Some(_) => panic!("Time limited app sent data for too long"),
            None => return,
        }
    }
}
