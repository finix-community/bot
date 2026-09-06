mod config;
mod event_handler;

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

    let http = Arc::new(HttpClient::new(cfg.discord_token.clone()));

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
        tokio::spawn(async move {
            if let Err(err) = event_handler::handle(http, event).await {
                error!(?err, "event handling blew up");
            }
        });
    }

    Ok(())
}
