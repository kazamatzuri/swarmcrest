// OAuth2 SSO support for GitHub and Google.
// Uses the `oauth2` crate for the authorization code flow with PKCE.

use axum::{
    extract::{ConnectInfo, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use oauth2::basic::{BasicErrorResponseType, BasicTokenType};
use oauth2::{
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    EndpointNotSet, EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
    RevocationErrorResponseType, Scope, StandardErrorResponse, StandardRevocableToken,
    StandardTokenIntrospectionResponse, StandardTokenResponse, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::db::Database;

// ── Type alias for a fully-configured OAuth2 client ─────────────────

type OAuthClient = Client<
    StandardErrorResponse<BasicErrorResponseType>,
    StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

// ── Pending OAuth flow store ────────────────────────────────────────

/// Stores PKCE verifiers keyed by CSRF state tokens.
/// The oauth2 crate generates the CSRF tokens and PKCE challenges;
/// we just need to hold onto the verifier until the callback arrives.
struct PendingFlow {
    pkce_verifier: PkceCodeVerifier,
    created_at: Instant,
}

const FLOW_TTL_SECS: u64 = 600; // 10 minutes

pub struct FlowStore {
    pending: RwLock<HashMap<String, PendingFlow>>,
}

impl FlowStore {
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
        }
    }

    async fn store(&self, csrf_token: &CsrfToken, verifier: PkceCodeVerifier) {
        let mut pending = self.pending.write().await;
        // Lazy cleanup of expired flows
        let cutoff = Instant::now() - std::time::Duration::from_secs(FLOW_TTL_SECS);
        pending.retain(|_, flow| flow.created_at > cutoff);
        pending.insert(
            csrf_token.secret().clone(),
            PendingFlow {
                pkce_verifier: verifier,
                created_at: Instant::now(),
            },
        );
    }

    async fn take(&self, state: &str) -> Option<PkceCodeVerifier> {
        let mut pending = self.pending.write().await;
        let flow = pending.remove(state)?;
        if flow.created_at.elapsed().as_secs() >= FLOW_TTL_SECS {
            return None;
        }
        Some(flow.pkce_verifier)
    }
}

// ── Shared types ────────────────────────────────────────────────────

pub(crate) struct OAuthUserInfo {
    pub(crate) provider: String,
    pub(crate) provider_user_id: String,
    pub(crate) username: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
}

/// Shared app state for OAuth endpoints.
pub struct OAuthState {
    pub db: Arc<Database>,
    pub flow_store: FlowStore,
    pub http_client: reqwest::Client,
    pub github_client: Option<OAuthClient>,
    pub google_client: Option<OAuthClient>,
    pub redirect_base: String,
    pub local_mode: bool,
}

impl OAuthState {
    pub fn new(cfg: &crate::config::Config, db: Arc<Database>) -> Self {
        let github_client = match (&cfg.github_client_id, &cfg.github_client_secret) {
            (Some(id), Some(secret)) => {
                let redirect =
                    format!("{}/api/auth/oauth/github/callback", cfg.oauth_redirect_base);
                Some(
                    Client::new(ClientId::new(id.clone()))
                        .set_client_secret(ClientSecret::new(secret.clone()))
                        .set_auth_uri(
                            AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
                                .unwrap(),
                        )
                        .set_token_uri(
                            TokenUrl::new(
                                "https://github.com/login/oauth/access_token".to_string(),
                            )
                            .unwrap(),
                        )
                        .set_redirect_uri(RedirectUrl::new(redirect).unwrap()),
                )
            }
            _ => None,
        };

        let google_client = match (&cfg.google_client_id, &cfg.google_client_secret) {
            (Some(id), Some(secret)) => {
                let redirect =
                    format!("{}/api/auth/oauth/google/callback", cfg.oauth_redirect_base);
                Some(
                    Client::new(ClientId::new(id.clone()))
                        .set_client_secret(ClientSecret::new(secret.clone()))
                        .set_auth_uri(
                            AuthUrl::new(
                                "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                            )
                            .unwrap(),
                        )
                        .set_token_uri(
                            TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                                .unwrap(),
                        )
                        .set_redirect_uri(RedirectUrl::new(redirect).unwrap()),
                )
            }
            _ => None,
        };

        if github_client.is_some() {
            tracing::info!("GitHub SSO enabled");
        }
        if google_client.is_some() {
            tracing::info!("Google SSO enabled");
        }

        Self {
            db,
            flow_store: FlowStore::new(),
            http_client: reqwest::Client::builder()
                .user_agent("SwarmCrest")
                .build()
                .expect("HTTP client should build"),
            github_client,
            google_client,
            redirect_base: cfg.oauth_redirect_base.clone(),
            local_mode: cfg.local_mode,
        }
    }
}

// ── Account resolution logic ────────────────────────────────────────

pub(crate) fn sanitize_username(input: &str) -> String {
    let clean: String = input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(30)
        .collect();
    if clean.len() < 3 {
        format!("user_{clean}")
    } else {
        clean
    }
}

pub(crate) async fn generate_unique_username(base: &str, db: &Database) -> Result<String, String> {
    let sanitized = sanitize_username(base);
    if !db
        .username_exists(&sanitized)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(sanitized);
    }
    for i in 1..100 {
        let candidate = format!("{}{i}", &sanitized[..sanitized.len().min(27)]);
        if !db
            .username_exists(&candidate)
            .await
            .map_err(|e| e.to_string())?
        {
            return Ok(candidate);
        }
    }
    Err("Could not generate unique username".to_string())
}

pub(crate) async fn find_or_create_user(
    db: &Database,
    info: OAuthUserInfo,
) -> Result<crate::db::User, String> {
    // 1. Check if this OAuth identity is already linked
    if let Some(oauth) = db
        .find_oauth_account(&info.provider, &info.provider_user_id)
        .await
        .map_err(|e| e.to_string())?
    {
        let user = db
            .get_user(oauth.user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Linked user not found")?;
        if let Some(ref url) = info.avatar_url {
            let _ = db.update_user_avatar(user.id, url).await;
        }
        return Ok(user);
    }

    // 2. Check if a user with this email already exists (account linking)
    if let Some(ref email) = info.email {
        if let Some(user) = db
            .get_user_by_email(email)
            .await
            .map_err(|e| e.to_string())?
        {
            db.create_oauth_account(
                user.id,
                &info.provider,
                &info.provider_user_id,
                info.username.as_deref(),
                info.email.as_deref(),
            )
            .await
            .map_err(|e| e.to_string())?;
            if let Some(ref url) = info.avatar_url {
                let _ = db.update_user_avatar(user.id, url).await;
            }
            return Ok(user);
        }
    }

    // 3. Create a new user
    let base_username = info
        .username
        .as_deref()
        .or(info.display_name.as_deref())
        .unwrap_or("user");
    let username = generate_unique_username(base_username, db).await?;
    let display = info.display_name.as_deref().unwrap_or(&username);
    let email = info.email.as_deref().unwrap_or("unknown@oauth");

    let user = db
        .create_user_oauth(&username, email, display, info.avatar_url.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    db.create_oauth_account(
        user.id,
        &info.provider,
        &info.provider_user_id,
        info.username.as_deref(),
        info.email.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(user)
}

// ── Provider availability endpoint ──────────────────────────────────

pub async fn auth_providers(State(oauth): State<Arc<OAuthState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "github": oauth.github_client.is_some(),
        "google": oauth.google_client.is_some(),
        "local_mode": oauth.local_mode,
    }))
}

// ── GitHub OAuth ────────────────────────────────────────────────────

pub async fn github_auth_start(State(oauth): State<Arc<OAuthState>>) -> impl IntoResponse {
    let Some(client) = &oauth.github_client else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "GitHub SSO not configured"})),
        )
            .into_response();
    };

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    oauth.flow_store.store(&csrf_token, pkce_verifier).await;

    let url = auth_url.to_string();
    (StatusCode::OK, Json(serde_json::json!({"url": url}))).into_response()
}

#[derive(Deserialize)]
pub struct OAuthCallback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub async fn github_auth_callback(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(oauth): State<Arc<OAuthState>>,
    Query(params): Query<OAuthCallback>,
) -> impl IntoResponse {
    let base = &oauth.redirect_base;

    if let Some(err) = &params.error {
        return Redirect::temporary(&format!(
            "{}/login?error=oauth_denied&detail={}",
            base,
            urlencoding::encode(err)
        ))
        .into_response();
    }

    if crate::rate_limit::check_auth_rate_limit(addr.ip()).is_err() {
        return Redirect::temporary(&format!("{}/login?error=rate_limited", base)).into_response();
    }

    let (Some(code), Some(state)) = (&params.code, &params.state) else {
        return Redirect::temporary(&format!("{}/login?error=missing_params", base))
            .into_response();
    };

    // Validate CSRF state and retrieve PKCE verifier
    let Some(pkce_verifier) = oauth.flow_store.take(state).await else {
        return Redirect::temporary(&format!("{}/login?error=invalid_state", base)).into_response();
    };

    let Some(client) = &oauth.github_client else {
        return Redirect::temporary(&format!("{}/login?error=not_configured", base))
            .into_response();
    };

    // Exchange authorization code for access token (oauth2 crate handles PKCE + token parsing)
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("HTTP client should build");

    let token_response = match client
        .exchange_code(AuthorizationCode::new(code.clone()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("GitHub token exchange failed: {e}");
            return Redirect::temporary(&format!("{}/login?error=token_exchange_failed", base))
                .into_response();
        }
    };

    let access_token = token_response.access_token().secret().to_string();

    // Fetch user profile (reqwest for API calls - oauth2 crate only handles the token exchange)
    #[derive(Deserialize)]
    struct GitHubUser {
        id: i64,
        login: String,
        name: Option<String>,
        avatar_url: Option<String>,
        email: Option<String>,
    }

    let gh_user: GitHubUser = match oauth
        .http_client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("Failed to parse GitHub user: {e}");
                return Redirect::temporary(&format!("{}/login?error=user_fetch_failed", base))
                    .into_response();
            }
        },
        Err(e) => {
            tracing::error!("GitHub user fetch failed: {e}");
            return Redirect::temporary(&format!("{}/login?error=user_fetch_failed", base))
                .into_response();
        }
    };

    // Fetch verified email if not in profile
    let email = if gh_user.email.is_some() {
        gh_user.email.clone()
    } else {
        #[derive(Deserialize)]
        struct GitHubEmail {
            email: String,
            primary: bool,
            verified: bool,
        }

        match oauth
            .http_client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
        {
            Ok(r) => {
                let emails: Vec<GitHubEmail> = r.json().await.unwrap_or_default();
                emails
                    .into_iter()
                    .find(|e| e.primary && e.verified)
                    .map(|e| e.email)
            }
            Err(_) => None,
        }
    };

    let email = email.unwrap_or_else(|| format!("{}@users.noreply.github.com", gh_user.login));

    let info = OAuthUserInfo {
        provider: "github".to_string(),
        provider_user_id: gh_user.id.to_string(),
        username: Some(gh_user.login),
        email: Some(email),
        display_name: gh_user.name,
        avatar_url: gh_user.avatar_url,
    };

    resolve_and_redirect(&oauth.db, info, base).await
}

// ── Google OAuth ────────────────────────────────────────────────────

pub async fn google_auth_start(State(oauth): State<Arc<OAuthState>>) -> impl IntoResponse {
    let Some(client) = &oauth.google_client else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Google SSO not configured"})),
        )
            .into_response();
    };

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    oauth.flow_store.store(&csrf_token, pkce_verifier).await;

    let url = auth_url.to_string();
    (StatusCode::OK, Json(serde_json::json!({"url": url}))).into_response()
}

pub async fn google_auth_callback(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(oauth): State<Arc<OAuthState>>,
    Query(params): Query<OAuthCallback>,
) -> impl IntoResponse {
    let base = &oauth.redirect_base;

    if let Some(err) = &params.error {
        return Redirect::temporary(&format!(
            "{}/login?error=oauth_denied&detail={}",
            base,
            urlencoding::encode(err)
        ))
        .into_response();
    }

    if crate::rate_limit::check_auth_rate_limit(addr.ip()).is_err() {
        return Redirect::temporary(&format!("{}/login?error=rate_limited", base)).into_response();
    }

    let (Some(code), Some(state)) = (&params.code, &params.state) else {
        return Redirect::temporary(&format!("{}/login?error=missing_params", base))
            .into_response();
    };

    let Some(pkce_verifier) = oauth.flow_store.take(state).await else {
        return Redirect::temporary(&format!("{}/login?error=invalid_state", base)).into_response();
    };

    let Some(client) = &oauth.google_client else {
        return Redirect::temporary(&format!("{}/login?error=not_configured", base))
            .into_response();
    };

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("HTTP client should build");

    let token_response = match client
        .exchange_code(AuthorizationCode::new(code.clone()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Google token exchange failed: {e}");
            return Redirect::temporary(&format!("{}/login?error=token_exchange_failed", base))
                .into_response();
        }
    };

    let access_token = token_response.access_token().secret().to_string();

    #[derive(Deserialize)]
    struct GoogleUser {
        id: String,
        email: Option<String>,
        verified_email: Option<bool>,
        name: Option<String>,
        picture: Option<String>,
    }

    let google_user: GoogleUser = match oauth
        .http_client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("Failed to parse Google user: {e}");
                return Redirect::temporary(&format!("{}/login?error=user_fetch_failed", base))
                    .into_response();
            }
        },
        Err(e) => {
            tracing::error!("Google user fetch failed: {e}");
            return Redirect::temporary(&format!("{}/login?error=user_fetch_failed", base))
                .into_response();
        }
    };

    let email = match (google_user.email, google_user.verified_email) {
        (Some(email), Some(true)) => Some(email),
        (Some(email), None) => Some(email),
        _ => None,
    };

    let base_username = email
        .as_deref()
        .and_then(|e| e.split('@').next())
        .or(google_user.name.as_deref())
        .unwrap_or("user");

    let info = OAuthUserInfo {
        provider: "google".to_string(),
        provider_user_id: google_user.id,
        username: Some(base_username.to_string()),
        email,
        display_name: google_user.name,
        avatar_url: google_user.picture,
    };

    resolve_and_redirect(&oauth.db, info, base).await
}

// ── Shared redirect helper ──────────────────────────────────────────

pub(crate) async fn resolve_and_redirect(
    db: &Database,
    info: OAuthUserInfo,
    base: &str,
) -> axum::response::Response {
    match find_or_create_user(db, info).await {
        Ok(user) => {
            let token = match crate::auth::create_token(user.id, &user.username, &user.role) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("JWT creation failed: {e}");
                    return Redirect::temporary(&format!("{}/login?error=internal", base))
                        .into_response();
                }
            };
            Redirect::temporary(&format!("{}/auth/callback?token={}", base, token)).into_response()
        }
        Err(e) => {
            tracing::error!("OAuth user resolution failed: {e}");
            Redirect::temporary(&format!("{}/login?error=account_error", base)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FlowStore tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_flow_store_roundtrip() {
        let store = FlowStore::new();
        let csrf = CsrfToken::new_random();
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let _ = challenge; // we only store the verifier

        let secret = csrf.secret().clone();
        store.store(&csrf, verifier).await;

        // Should retrieve successfully
        let result = store.take(&secret).await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_flow_store_single_use() {
        let store = FlowStore::new();
        let csrf = CsrfToken::new_random();
        let (_challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let secret = csrf.secret().clone();

        store.store(&csrf, verifier).await;

        // First take succeeds
        assert!(store.take(&secret).await.is_some());
        // Second take fails (token consumed)
        assert!(store.take(&secret).await.is_none());
    }

    #[tokio::test]
    async fn test_flow_store_invalid_state() {
        let store = FlowStore::new();
        assert!(store.take("nonexistent-state").await.is_none());
    }

    #[tokio::test]
    async fn test_flow_store_multiple_flows() {
        let store = FlowStore::new();

        let csrf1 = CsrfToken::new_random();
        let csrf2 = CsrfToken::new_random();
        let (_, v1) = PkceCodeChallenge::new_random_sha256();
        let (_, v2) = PkceCodeChallenge::new_random_sha256();

        let s1 = csrf1.secret().clone();
        let s2 = csrf2.secret().clone();

        store.store(&csrf1, v1).await;
        store.store(&csrf2, v2).await;

        // Both should be independently retrievable
        assert!(store.take(&s1).await.is_some());
        assert!(store.take(&s2).await.is_some());

        // Neither should be retrievable again
        assert!(store.take(&s1).await.is_none());
        assert!(store.take(&s2).await.is_none());
    }

    // ── sanitize_username tests ─────────────────────────────────────

    #[test]
    fn test_sanitize_username_basic() {
        assert_eq!(sanitize_username("john_doe"), "john_doe");
        assert_eq!(sanitize_username("JohnDoe123"), "JohnDoe123");
    }

    #[test]
    fn test_sanitize_username_strips_special_chars() {
        assert_eq!(sanitize_username("john.doe@email"), "johndoeemail");
        assert_eq!(sanitize_username("user-name!"), "username");
        assert_eq!(sanitize_username("hello world"), "helloworld");
    }

    #[test]
    fn test_sanitize_username_too_short_gets_prefix() {
        assert_eq!(sanitize_username("ab"), "user_ab");
        assert_eq!(sanitize_username("x"), "user_x");
        assert_eq!(sanitize_username(""), "user_");
    }

    #[test]
    fn test_sanitize_username_truncates_long() {
        let long = "a".repeat(50);
        let result = sanitize_username(&long);
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_sanitize_username_special_only() {
        // All special chars stripped, result too short
        assert_eq!(sanitize_username("@#$"), "user_");
    }

    // ── generate_unique_username tests ──────────────────────────────

    #[tokio::test]
    async fn test_generate_unique_username_no_conflict() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();
        let name = generate_unique_username("testuser", &db).await.unwrap();
        assert_eq!(name, "testuser");
    }

    #[tokio::test]
    async fn test_generate_unique_username_with_conflict() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        // Create a user with the base name
        db.create_user("testuser", "a@b.com", "hash", "Test")
            .await
            .unwrap();

        let name = generate_unique_username("testuser", &db).await.unwrap();
        assert_eq!(name, "testuser1");
    }

    #[tokio::test]
    async fn test_generate_unique_username_multiple_conflicts() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        db.create_user("testuser", "a@b.com", "hash", "Test")
            .await
            .unwrap();
        db.create_user("testuser1", "b@b.com", "hash", "Test")
            .await
            .unwrap();
        db.create_user("testuser2", "c@b.com", "hash", "Test")
            .await
            .unwrap();

        let name = generate_unique_username("testuser", &db).await.unwrap();
        assert_eq!(name, "testuser3");
    }

    // ── find_or_create_user tests ───────────────────────────────────

    #[tokio::test]
    async fn test_find_or_create_user_new_user() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        let info = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "12345".to_string(),
            username: Some("octocat".to_string()),
            email: Some("octocat@github.com".to_string()),
            display_name: Some("The Octocat".to_string()),
            avatar_url: Some("https://avatars.githubusercontent.com/u/583231".to_string()),
        };

        let user = find_or_create_user(&db, info).await.unwrap();
        assert_eq!(user.username, "octocat");
        assert_eq!(user.email, "octocat@github.com");
        assert_eq!(user.display_name, Some("The Octocat".to_string()));
        assert_eq!(
            user.avatar_url,
            Some("https://avatars.githubusercontent.com/u/583231".to_string())
        );
        assert!(user.password_hash.is_none()); // OAuth user, no password

        // Verify OAuth account was created
        let oauth = db.find_oauth_account("github", "12345").await.unwrap();
        assert!(oauth.is_some());
        assert_eq!(oauth.unwrap().user_id, user.id);
    }

    #[tokio::test]
    async fn test_find_or_create_user_existing_oauth_link() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        // First login creates the user
        let info1 = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "12345".to_string(),
            username: Some("octocat".to_string()),
            email: Some("octocat@github.com".to_string()),
            display_name: Some("The Octocat".to_string()),
            avatar_url: None,
        };
        let user1 = find_or_create_user(&db, info1).await.unwrap();

        // Second login with same provider+ID finds the existing user
        let info2 = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "12345".to_string(),
            username: Some("octocat".to_string()),
            email: Some("octocat@github.com".to_string()),
            display_name: Some("New Name".to_string()),
            avatar_url: Some("https://new-avatar.com".to_string()),
        };
        let user2 = find_or_create_user(&db, info2).await.unwrap();

        assert_eq!(user1.id, user2.id); // same user
    }

    #[tokio::test]
    async fn test_find_or_create_user_email_linking() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        // Create a password-based user first
        db.create_user("alice", "alice@example.com", "hashedpw", "Alice")
            .await
            .unwrap();

        // OAuth login with same email should link, not create a new user
        let info = OAuthUserInfo {
            provider: "google".to_string(),
            provider_user_id: "google-id-1".to_string(),
            username: Some("alice_google".to_string()),
            email: Some("alice@example.com".to_string()),
            display_name: Some("Alice G".to_string()),
            avatar_url: None,
        };
        let user = find_or_create_user(&db, info).await.unwrap();

        assert_eq!(user.username, "alice"); // linked to existing user
        assert_eq!(user.email, "alice@example.com");

        // Verify OAuth account is linked
        let oauth = db
            .find_oauth_account("google", "google-id-1")
            .await
            .unwrap();
        assert!(oauth.is_some());
        assert_eq!(oauth.unwrap().user_id, user.id);
    }

    #[tokio::test]
    async fn test_find_or_create_user_multiple_providers() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        // Login via GitHub
        let gh_info = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "gh-123".to_string(),
            username: Some("devuser".to_string()),
            email: Some("dev@example.com".to_string()),
            display_name: Some("Dev User".to_string()),
            avatar_url: None,
        };
        let user_gh = find_or_create_user(&db, gh_info).await.unwrap();

        // Login via Google with same email should link to same user
        let google_info = OAuthUserInfo {
            provider: "google".to_string(),
            provider_user_id: "goog-456".to_string(),
            username: Some("devuser_google".to_string()),
            email: Some("dev@example.com".to_string()),
            display_name: Some("Dev User".to_string()),
            avatar_url: None,
        };
        let user_google = find_or_create_user(&db, google_info).await.unwrap();

        assert_eq!(user_gh.id, user_google.id);

        // Both OAuth accounts should exist
        assert!(db
            .find_oauth_account("github", "gh-123")
            .await
            .unwrap()
            .is_some());
        assert!(db
            .find_oauth_account("google", "goog-456")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_find_or_create_user_no_email() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        let info = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "99999".to_string(),
            username: Some("privateemail".to_string()),
            email: None,
            display_name: None,
            avatar_url: None,
        };
        let user = find_or_create_user(&db, info).await.unwrap();
        assert_eq!(user.username, "privateemail");
        assert_eq!(user.email, "unknown@oauth");
    }

    #[tokio::test]
    async fn test_find_or_create_user_username_conflict() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        // Create existing user with the same username
        db.create_user("octocat", "existing@example.com", "hash", "Existing")
            .await
            .unwrap();

        // OAuth user with same username but different email should get a numbered suffix
        let info = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "55555".to_string(),
            username: Some("octocat".to_string()),
            email: Some("different@example.com".to_string()),
            display_name: None,
            avatar_url: None,
        };
        let user = find_or_create_user(&db, info).await.unwrap();
        assert_eq!(user.username, "octocat1"); // conflict resolved with suffix
    }

    #[tokio::test]
    async fn test_find_or_create_user_avatar_updated_on_relogin() {
        sqlx::any::install_default_drivers();
        let db = crate::db::Database::new("sqlite::memory:").await.unwrap();

        // First login without avatar
        let info1 = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "77".to_string(),
            username: Some("avataruser".to_string()),
            email: Some("avatar@test.com".to_string()),
            display_name: None,
            avatar_url: None,
        };
        let user1 = find_or_create_user(&db, info1).await.unwrap();
        assert!(user1.avatar_url.is_none());

        // Second login with avatar should update it
        let info2 = OAuthUserInfo {
            provider: "github".to_string(),
            provider_user_id: "77".to_string(),
            username: Some("avataruser".to_string()),
            email: Some("avatar@test.com".to_string()),
            display_name: None,
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        };
        let _ = find_or_create_user(&db, info2).await.unwrap();

        // Check avatar was updated
        let refreshed = db.get_user(user1.id).await.unwrap().unwrap();
        assert_eq!(
            refreshed.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
    }
}
