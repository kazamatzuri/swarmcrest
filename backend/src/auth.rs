// Authentication: password hashing, JWT tokens, API key auth, and middleware.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{ConnectInfo, FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use std::net::SocketAddr;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::db::Database;

// ── JWT ──────────────────────────────────────────────────────────────

/// JWT secret – in production this should come from an env var.
fn jwt_secret() -> Vec<u8> {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "infon-dev-secret-change-in-production".to_string())
        .into_bytes()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64, // user id
    pub username: String,
    pub role: String,
    pub exp: usize, // expiry (unix timestamp)
    #[serde(default)]
    pub scopes: Option<String>, // None for JWT auth, Some("bots:read,...") for API tokens
}

pub fn create_token(user_id: i64, username: &str, role: &str) -> Result<String, String> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration,
        scopes: None,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&jwt_secret()),
    )
    .map_err(|e| format!("Failed to create token: {e}"))
}

pub fn verify_token(token: &str) -> Result<Claims, String> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&jwt_secret()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Invalid token: {e}"))
}

/// Check whether the given claims include a required scope.
/// JWT tokens (scopes == None) have full access; API tokens must list the scope.
pub fn has_scope(claims: &Claims, required: &str) -> bool {
    match &claims.scopes {
        None => true, // JWT tokens have full access
        Some(scopes) => scopes.split(',').any(|s| s.trim() == required),
    }
}

// ── Password hashing ─────────────────────────────────────────────────

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Failed to hash password: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| format!("Invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ── API token helper ─────────────────────────────────────────────────

/// Hash a raw API token with SHA-256 (same algorithm used in api/mod.rs).
fn hash_api_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Try to authenticate via API token. Returns Claims if the token is valid.
async fn try_api_token_auth(token: &str, parts: &Parts) -> Option<Claims> {
    if !token.starts_with("infon_") {
        return None;
    }

    let db = parts.extensions.get::<Arc<Database>>()?;
    let token_hash = hash_api_token(token);

    let api_token = db.get_api_token_by_hash(&token_hash).await.ok()??;

    // Update last_used_at (fire-and-forget)
    let _ = db.update_token_last_used(api_token.id).await;

    // Look up the user to build Claims
    let user = db.get_user(api_token.user_id).await.ok()??;

    Some(Claims {
        sub: user.id,
        username: user.username,
        role: user.role,
        // API tokens don't expire via JWT, use a far-future expiry
        exp: (chrono::Utc::now().timestamp() + 86400) as usize,
        scopes: Some(api_token.scopes.clone()),
    })
}

/// Build synthetic claims for local mode (no real auth).
fn local_mode_claims() -> Claims {
    Claims {
        sub: crate::config::LOCAL_USER_ID,
        username: crate::config::LOCAL_USERNAME.to_string(),
        role: "user".to_string(),
        exp: (chrono::Utc::now().timestamp() + 86400 * 365) as usize,
        scopes: None, // full access
    }
}

// ── Axum extractor: AuthUser ─────────────────────────────────────────

/// Extracts the authenticated user from the Authorization header.
/// Supports both JWT tokens and API keys (prefixed with "infon_").
/// Usage: `AuthUser(claims)` in handler parameters.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // In local mode, skip authentication entirely and use the local user
        if crate::config::is_local_mode() {
            return Ok(AuthUser(local_mode_claims()));
        }

        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "Missing Authorization header"})),
                )
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid Authorization header format"})),
            )
        })?;

        // Try JWT first
        if let Ok(claims) = verify_token(token) {
            return Ok(AuthUser(claims));
        }

        // Fall back to API token auth
        if let Some(claims) = try_api_token_auth(token, parts).await {
            return Ok(AuthUser(claims));
        }

        Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid token"})),
        ))
    }
}

/// Optional auth extractor – does not reject if no token is present.
/// Supports both JWT tokens and API keys (prefixed with "infon_").
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Claims>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // In local mode, always provide the local user
        if crate::config::is_local_mode() {
            return Ok(OptionalAuthUser(Some(local_mode_claims())));
        }

        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok());

        let Some(header) = auth_header else {
            return Ok(OptionalAuthUser(None));
        };

        let Some(token) = header.strip_prefix("Bearer ") else {
            return Ok(OptionalAuthUser(None));
        };

        // Try JWT first
        if let Ok(claims) = verify_token(token) {
            return Ok(OptionalAuthUser(Some(claims)));
        }

        // Fall back to API token auth
        if let Some(claims) = try_api_token_auth(token, parts).await {
            return Ok(OptionalAuthUser(Some(claims)));
        }

        Ok(OptionalAuthUser(None))
    }
}

// ── Auth API handlers ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Serialize)]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
    pub created_at: String,
}

pub async fn register(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(db): State<Arc<Database>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    if let Err(e) = crate::rate_limit::check_auth_rate_limit(addr.ip()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": e})),
        )
            .into_response();
    }

    if req.username.is_empty() || req.password.is_empty() || req.email.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "username, email, and password are required"})),
        )
            .into_response();
    }

    if req.username.len() < 3 || req.username.len() > 30 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "username must be 3-30 characters"})),
        )
            .into_response();
    }

    if req.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "password must be at least 8 characters"})),
        )
            .into_response();
    }

    let password_hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Password hash error: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response();
        }
    };

    let display_name = req.display_name.unwrap_or_else(|| req.username.clone());

    match db
        .create_user(&req.username, &req.email, &password_hash, &display_name)
        .await
    {
        Ok(user) => {
            let token = match create_token(user.id, &user.username, &user.role) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Token creation error: {e}");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Internal error"})),
                    )
                        .into_response();
                }
            };
            (
                StatusCode::CREATED,
                Json(serde_json::json!(AuthResponse {
                    token,
                    user: UserPublic {
                        id: user.id,
                        username: user.username,
                        email: user.email,
                        display_name: user.display_name,
                        role: user.role,
                        created_at: user.created_at,
                    },
                })),
            )
                .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE") {
                (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": "Username or email already taken"})),
                )
                    .into_response()
            } else {
                tracing::error!("DB error in register: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Internal error"})),
                )
                    .into_response()
            }
        }
    }
}

pub async fn login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(db): State<Arc<Database>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    if let Err(e) = crate::rate_limit::check_auth_rate_limit(addr.ip()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": e})),
        )
            .into_response();
    }

    let user = match db.get_user_by_username(&req.username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid username or password"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("DB error in login: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response();
        }
    };

    let Some(ref password_hash) = user.password_hash else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "This account uses OAuth login"})),
        )
            .into_response();
    };

    match verify_password(&req.password, password_hash) {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid username or password"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Password verify error: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response();
        }
    }

    let token = match create_token(user.id, &user.username, &user.role) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Token creation error: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(serde_json::json!(AuthResponse {
            token,
            user: UserPublic {
                id: user.id,
                username: user.username,
                email: user.email,
                display_name: user.display_name,
                role: user.role,
                created_at: user.created_at,
            },
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
}

pub async fn update_profile(
    AuthUser(claims): AuthUser,
    State(db): State<Arc<Database>>,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    match db
        .update_user(claims.sub, req.display_name.as_deref(), req.bio.as_deref())
        .await
    {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(serde_json::json!(UserPublic {
                id: user.id,
                username: user.username,
                email: user.email,
                display_name: user.display_name,
                role: user.role,
                created_at: user.created_at,
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "User not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

/// Auto-login endpoint for local mode. Returns a JWT token for the local user
/// without requiring credentials. Returns 404 if not in local mode.
pub async fn local_login(State(db): State<Arc<Database>>) -> impl IntoResponse {
    if !crate::config::is_local_mode() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    // Find or use the local user
    match db
        .get_user_by_username(crate::config::LOCAL_USERNAME)
        .await
    {
        Ok(Some(user)) => {
            let token = match create_token(user.id, &user.username, &user.role) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Token creation error: {e}");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Internal error"})),
                    )
                        .into_response();
                }
            };
            (
                StatusCode::OK,
                Json(serde_json::json!(AuthResponse {
                    token,
                    user: UserPublic {
                        id: user.id,
                        username: user.username,
                        email: user.email,
                        display_name: user.display_name,
                        role: user.role,
                        created_at: user.created_at,
                    },
                })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Local user not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

/// Check whether local mode is enabled (used by frontend to show/hide login).
pub async fn local_mode_status() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "local_mode": crate::config::is_local_mode()
        })),
    )
        .into_response()
}

pub async fn me(AuthUser(claims): AuthUser, State(db): State<Arc<Database>>) -> impl IntoResponse {
    match db.get_user(claims.sub).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(serde_json::json!(UserPublic {
                id: user.id,
                username: user.username,
                email: user.email,
                display_name: user.display_name,
                role: user.role,
                created_at: user.created_at,
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "User not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "testpassword123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }

    #[test]
    fn test_jwt_create_and_verify() {
        let token = create_token(1, "testuser", "user").unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "user");
    }

    #[test]
    fn test_jwt_invalid_token() {
        assert!(verify_token("invalid.token.here").is_err());
    }

    #[test]
    fn test_jwt_claims_have_no_scopes() {
        let token = create_token(1, "testuser", "user").unwrap();
        let claims = verify_token(&token).unwrap();
        assert!(claims.scopes.is_none());
        // JWT tokens (no scopes) have full access
        assert!(has_scope(&claims, "bots:write"));
        assert!(has_scope(&claims, "matches:write"));
    }

    #[test]
    fn test_has_scope_with_api_token_scopes() {
        let claims = Claims {
            sub: 1,
            username: "testuser".to_string(),
            role: "user".to_string(),
            exp: 9999999999,
            scopes: Some("bots:read,matches:read,leaderboard:read".to_string()),
        };
        assert!(has_scope(&claims, "bots:read"));
        assert!(has_scope(&claims, "matches:read"));
        assert!(has_scope(&claims, "leaderboard:read"));
        assert!(!has_scope(&claims, "bots:write"));
        assert!(!has_scope(&claims, "matches:write"));
    }
}
