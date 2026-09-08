use eros::Context;
use serde::Deserialize;

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
}

#[derive(Deserialize)]
struct DiscordConnection {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    verified: bool,
}

pub struct VerifiedIdentity {
    pub discord_id: u64,
    pub github_login: String,
}

pub async fn fetch(access_token: &str) -> eros::Result<VerifiedIdentity> {
    let http = reqwest::Client::new();

    let discord_user: DiscordUser = http
        .get("https://discord.com/api/users/@me")
        .bearer_auth(access_token)
        .send()
        .await
        .context("fetching discord user")?
        .json()
        .await
        .context("parsing discord user response")?;

    let connections: Vec<DiscordConnection> = http
        .get("https://discord.com/api/users/@me/connections")
        .bearer_auth(access_token)
        .send()
        .await
        .context("fetching discord connections")?
        .json()
        .await
        .context("parsing discord connections response")?;

    let github_login = connections
        .into_iter()
        .find(|c| c.kind == "github" && c.verified)
        .map(|c| c.name)
        .context("no verified github connection found")?;

    let discord_id = discord_user
        .id
        .parse::<u64>()
        .context("discord user id is not a number")?;

    Ok(VerifiedIdentity {
        discord_id,
        github_login,
    })
}
