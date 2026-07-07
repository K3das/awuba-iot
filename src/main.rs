pub mod aruba_proto {
    pub mod telemetry {
        include!(concat!(env!("OUT_DIR"), "/aruba_telemetry.rs"));
    }
}

mod aruba;
mod config;
mod hass;
mod protocols;
mod utils;

extern crate config as config_parser;

use crate::{
    aruba_proto::telemetry,
    config::Config,
    protocols::{ParserRegistry, ProtocolType},
};
use anyhow::Context;
use axum::{
    Router,
    extract::{
        State,
        connect_info::ConnectInfo,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::any,
};
use hashlink::LruCache;
use prost::Message as _;
use rumqttc::{AsyncClient, MqttOptions};
use std::{
    net::SocketAddr,
    ops::ControlFlow,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

type MacSet = LruCache<(ProtocolType, [u8; 6]), ()>;

#[derive(Clone)]
struct AppState {
    pub protocols: Arc<Mutex<ParserRegistry>>,
    pub config: Arc<Config>,
    pub mqtt: AsyncClient,
    pub published_macs: Arc<Mutex<MacSet>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config: Config = config_parser::Config::builder()
        .add_source(config_parser::File::with_name("config").required(false))
        .add_source(
            config_parser::Environment::with_prefix("AWUBA")
                .separator("__")
                .list_separator(",")
                .convert_case(config_parser::Case::Snake),
        )
        .build()?
        .try_deserialize::<Config>()?;

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .context("couldn't parse RUST_LOG")?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut mqtt_options = MqttOptions::new(
        config.mqtt.client_id.clone(),
        config.mqtt.broker_host.clone(),
        config.mqtt.broker_port,
    );
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    match (&config.mqtt.broker_username, &config.mqtt.broker_password) {
        (Some(username), Some(password)) => {
            mqtt_options.set_credentials(username.clone(), password.clone());
        }
        (None, None) => {}
        _ => {
            tracing::warn!(
                "Only one of mqtt.broker_username or mqtt.broker_password is set, but not the other."
            );
        }
    }

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    let state = AppState {
        protocols: Arc::new(ParserRegistry::default_handlers().into()),
        config: Arc::new(config.clone()),
        mqtt: client,
        published_macs: Arc::new(Mutex::new(LruCache::new(512))),
    };

    let published_macs = Arc::clone(&state.published_macs);
    tokio::spawn(async move {
        loop {
            if let Err(e) = eventloop.poll().await {
                tracing::error!("MQTT connection error: {e:?}");
                published_macs.lock().unwrap().clear();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    });

    let app = Router::new()
        .route("/ws", any(handle_ws_upgrade))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.app.listen_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    tracing::info!("{addr} initiated websocket");
    ws.on_upgrade(move |socket| handle_socket(state, socket, addr))
}

async fn handle_socket(state: AppState, mut socket: WebSocket, who: SocketAddr) {
    let state = &state.clone();
    while let Some(msg_result) = socket.recv().await {
        match msg_result {
            Ok(msg) => {
                if handle_ws_message(state, msg, who).await.is_break() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("error receiving websocket message from {who}: {e}");
                break;
            }
        }
    }
    tracing::info!("websocket {who} ended");
}

async fn handle_ws_message(state: &AppState, msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Binary(d) => {
            let telem = match telemetry::Telemetry::decode(d.as_ref()) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("failed to decode telemetry from {who}: {e}");
                    return ControlFlow::Break(());
                }
            };

            if telem.meta.access_token() != state.config.app.access_token {
                tracing::info!("invalid token from {who}");
                return ControlFlow::Break(());
            }
            tracing::debug!("{who} sent {:?} {:?}", telem.meta.nb_topic(), telem);

            if let Err(e) = aruba::handle_aruba_nb(state, telem).await {
                tracing::error!("error processing telemetry from {who}: {e:#}");
            }
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                tracing::info!(
                    "{who} sent close with code {} and reason `{}`",
                    cf.code,
                    cf.reason
                );
            } else {
                tracing::info!("{who} sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }
        _ => {}
    }
    ControlFlow::Continue(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
