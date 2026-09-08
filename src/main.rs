mod config;
mod discord_identity;
mod event_handler;
mod github_client;
mod oauth_server;
mod storage;

use eros::Context;
use std::sync::Arc;
use tracing::{error, info, warn};
use twilight_gateway::{
    Config as GatewayConfig, EventTypeFlags, Intents, Shard, ShardId, StreamExt,
};
use twilight_http::Client as HttpClient;

#[tokio::main]
async fn main() -> eros::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finix_bot=info,twilight=info".into()),
        )
        .init();

    let cfg = config::Config::from_env().context("starting bot")?;
    info!(guild_id = cfg.guild_id, github_org = %cfg.github_org, "starting finix-bot");

    let storage = Arc::new(storage::Storage::connect(&cfg.database_url).await?);
    let http = Arc::new(HttpClient::new(cfg.discord_token.clone()));

    {
        let app_state = Arc::new(oauth_server::AppState {
            config: cfg.clone(),
            storage: Arc::clone(&storage),
        });
        let listen_addr = cfg.oauth_listen_addr.clone();
        tokio::spawn(async move {
            let router = oauth_server::router(app_state);
            let listener = tokio::net::TcpListener::bind(&listen_addr)
                .await
                .unwrap_or_else(|_| panic!("binding {listen_addr} for the oauth callback"));
            info!(%listen_addr, "oauth callback server listening");
            if let Err(err) = axum::serve(listener, router).await {
                error!(?err, "oauth server crashed");
            }
        });
    }

    let intents = Intents::GUILDS | Intents::GUILD_MEMBERS;
    let gateway_config = GatewayConfig::new(cfg.discord_token.clone(), intents);
    let mut shard = Shard::with_config(ShardId::ONE, gateway_config);

    info!("connecting to the Discord gateway");

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(event) => event,
            Err(source) => {
                warn!(?source, "shard error, continuing");
                continue;
            }
        };

        let http = Arc::clone(&http);
        let storage = Arc::clone(&storage);
        tokio::spawn(async move {
            if let Err(err) = event_handler::handle(http, storage, event).await {
                error!(?err, "event handling blew up");
            }
        });
    }

    Ok(())
}
