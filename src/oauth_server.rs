use crate::config::Config;
use crate::storage::Storage;
use axum::{
    Router,
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use eros::{Context, bail};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    RedirectUrl, Scope, TokenUrl, TokenResponse, basic::BasicClient,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tracing::warn;

/// How long an issued CSRF state token remains valid. Anyone who hasn't
/// completed the Discord authorize step within this window has to start over.
const STATE_TTL: Duration = Duration::from_secs(10 * 60);

pub struct AppState {
    pub config: Config,
    pub storage: Arc<Storage>,
    /// CSRF state tokens we've handed out via `/oauth/discord/start`, along
    /// with when they were issued. A callback is only honored if its `state`
    /// is found (and then removed) here, which is what makes it a one-time,
    /// server-verified value rather than data an attacker could forge.
    pending_states: StdMutex<HashMap<String, Instant>>,
}

impl AppState {
    pub fn new(config: Config, storage: Arc<Storage>) -> Self {
        Self {
            config,
            storage,
            pending_states: StdMutex::new(HashMap::new()),
        }
    }
}

fn prune_expired(pending: &mut HashMap<String, Instant>) {
    let now = Instant::now();
    pending.retain(|_, issued_at| now.duration_since(*issued_at) < STATE_TTL);
}

type DiscordOAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
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
        .route("/oauth/discord/start", get(discord_start))
        .route("/oauth/discord/callback", get(discord_callback))
        .with_state(state)
}

/// Entry point users hit to begin linking their account
async fn discord_start(State(state): State<Arc<AppState>>) -> Response {
    let client = match discord_oauth_client(&state.config) {
        Ok(client) => client,
        Err(err) => {
            warn!(?err, "building discord oauth client failed");
            return Html("<h1>Something went wrong</h1><p>Try again later.</p>".to_string())
                .into_response();
        }
    };

    let (auth_url, csrf) = build_auth_url(&client);

    {
        let mut pending = state
            .pending_states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        prune_expired(&mut pending);
        pending.insert(csrf.secret().clone(), Instant::now());
    }

    Redirect::to(&auth_url).into_response()
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
    {
        let mut pending = state
            .pending_states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        prune_expired(&mut pending);
        if pending.remove(&params.state).is_none() {
            bail!("oauth callback presented an unknown, expired, or already-used state token");
        }
    }

    let client = discord_oauth_client(&state.config)?;
    let oauth_http_client = oauth2::reqwest::Client::new();

    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&oauth_http_client)
        .await
        .context("exchanging oauth code for token")?;

    let access_token = token.access_token().secret().clone();
    let refresh_token = token
        .refresh_token()
        .map(|t| t.secret().clone())
        .unwrap_or_default();

    let identity = crate::discord_identity::fetch(&access_token)
        .await
        .context("fetching verified discord identity")?;

    state
        .storage
        .upsert_link(identity.discord_id, &identity.github_login, &refresh_token)
        .await
        .context("saving linked account")?;

    Ok(())
}
