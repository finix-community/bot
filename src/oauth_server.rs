use crate::config::Config;
use crate::storage::Storage;
use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use eros::Context;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    RedirectUrl, Scope, TokenUrl, TokenResponse, basic::BasicClient,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::warn;

pub struct AppState {
    pub config: Config,
    pub storage: Arc<Storage>,
}

type DiscordOAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    #[allow(dead_code)]
    state: String,
}

pub fn discord_oauth_client(cfg: &Config) -> eros::Result<DiscordOAuthClient> {
    Ok(BasicClient::new(ClientId::new(cfg.discord_client_id.clone()))
        .set_client_secret(ClientSecret::new(cfg.discord_client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string())
                .context("parsing discord auth url")?,
        )
        .set_token_uri(
            TokenUrl::new("https://discord.com/api/oauth2/token".to_string())
                .context("parsing discord token url")?,
        )
        .set_redirect_uri(
            RedirectUrl::new(cfg.oauth_redirect_uri.clone()).context("parsing redirect uri")?,
        ))
}

pub fn build_auth_url(client: &DiscordOAuthClient) -> (String, CsrfToken) {
    let (url, csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .add_scope(Scope::new("connections".to_string()))
        .url();
    (url.to_string(), csrf)
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/oauth/discord/callback", get(discord_callback))
        .with_state(state)
}

async fn discord_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CallbackParams>,
) -> Html<String> {
    match handle_callback(state, params).await {
        Ok(()) => Html(
            "<h1>Connected!</h1><p>You can go back to Discord, your role will update shortly.</p>"
                .into(),
        ),
        Err(err) => {
            warn!(?err, "oauth callback failed");
            Html("<h1>Something went wrong</h1><p>Try connecting again.</p>".into())
        }
    }
}

async fn handle_callback(state: Arc<AppState>, params: CallbackParams) -> eros::Result<()> {
    let client = discord_oauth_client(&state.config)?;
    let http_client = oauth2::reqwest::Client::new();

    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&http_client)
        .await
        .context("exchanging oauth code for token")?;

    let access_token = token.access_token().secret().clone();
    let refresh_token = token
        .refresh_token()
        .map(|t| t.secret().clone())
        .unwrap_or_default();

    let _ = (access_token, refresh_token);

    Ok(())
}
