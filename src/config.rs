use eros::Context;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub oauth_redirect_uri: String,
    pub github_org: String,
    pub guild_id: u64,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub database_url: String,
    pub resync_interval_secs: u64,
}

fn require(key: &str) -> eros::Result<String> {
    env::var(key).with_context(|| format!("missing environment variable: {key}"))
}

impl Config {
    pub fn from_env() -> eros::Result<Self> {
        let guild_id: u64 = require("GUILD_ID")?
            .parse()
            .context("GUILD_ID must be a number (snowflake)")?;

        Ok(Self {
            discord_token: require("DISCORD_TOKEN")?,
            discord_client_id: require("DISCORD_CLIENT_ID")?,
            discord_client_secret: require("DISCORD_CLIENT_SECRET")?,
            oauth_redirect_uri: require("OAUTH_REDIRECT_URI")?,
            github_org: env::var("GITHUB_ORG").unwrap_or_else(|_| "finix-community".to_string()),
            guild_id,
            github_client_id: require("GITHUB_CLIENT_ID")?,
            github_client_secret: require("GITHUB_CLIENT_SECRET")?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://finix-bot.sqlite".to_string()),
            resync_interval_secs: env::var("RESYNC_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
        })
    }
}
