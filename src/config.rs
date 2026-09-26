use eros::Context;
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub oauth_redirect_uri: String,
    pub github_org: String,
    pub guild_id: u64,
    pub database_url: String,
    pub resync_interval_secs: u64,
    pub oauth_listen_addr: String,
    pub repo_role_map: HashMap<String, u64>,
}

fn require(key: &str) -> eros::Result<String> {
    Ok(env::var(key).with_context(|| format!("missing environment variable: {key}"))?)
}

/// Parses `REPO_ROLE_MAP`, a JSON object mapping GitHub repo names to
/// Discord role IDs, e.g.: `{"finix": "123456789012345678", "community-modules": "234567890123456789"}`
fn parse_repo_role_map(raw: &str) -> eros::Result<HashMap<String, u64>> {
    let raw_map: HashMap<String, String> = serde_json::from_str(raw).context(
        "REPO_ROLE_MAP must be a JSON object of repo name -> Discord role ID given as a string",
    )?;

    raw_map
        .into_iter()
        .map(|(repo, role_id)| {
            let role_id = role_id.parse::<u64>().with_context(|| {
                format!("REPO_ROLE_MAP entry for repo '{repo}' is not a valid role snowflake")
            })?;
            Ok((repo, role_id))
        })
        .collect()
}

impl Config {
    pub fn from_env() -> eros::Result<Self> {
        let guild_id = require("GUILD_ID")?
            .parse::<u64>()
            .context("GUILD_ID must be a number (snowflake)")?;

        let repo_role_map = match env::var("REPO_ROLE_MAP") {
            Ok(raw) => parse_repo_role_map(&raw).context("parsing REPO_ROLE_MAP")?,
            Err(_) => HashMap::new(),
        };

        Ok(Self {
            discord_token: require("DISCORD_TOKEN")?,
            discord_client_id: require("DISCORD_CLIENT_ID")?,
            discord_client_secret: require("DISCORD_CLIENT_SECRET")?,
            oauth_redirect_uri: require("OAUTH_REDIRECT_URI")?,
            github_org: env::var("GITHUB_ORG").unwrap_or_else(|_| "finix-community".to_string()),
            guild_id,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://finix-bot.sqlite".to_string()),
            resync_interval_secs: env::var("RESYNC_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            oauth_listen_addr: env::var("OAUTH_LISTEN_ADDR").unwrap_or_else(|_| {
                let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
                format!("0.0.0.0:{port}")
            }),
            repo_role_map,
        })
    }

    /// Discord role IDs earned by having contributed to any of `repos`.
    pub fn roles_for_repos<'a>(&self, repos: impl IntoIterator<Item = &'a str>) -> Vec<u64> {
        repos
            .into_iter()
            .filter_map(|repo| self.repo_role_map.get(repo).copied())
            .collect()
    }
}
