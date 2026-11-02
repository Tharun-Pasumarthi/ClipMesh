use crate::error::ApiError;
use crate::models::{AuthResponse, LoginRequest, RefreshTokenRequest, RegisterUserRequest};
use crate::state::{AppState, UserRecord};
use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Register a new user account (or create anonymous account if no email provided)
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterUserRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let user_id = Uuid::new_v4();
    let email_copy = payload.email.clone();

    // If email is provided, ensure uniqueness
    if let Some(ref email) = payload.email {
        let users = state.users.read();
        for u in users.values() {
            if u.email.as_ref() == Some(email) {
                return Err(ApiError::Conflict("Email already registered".to_string()));
            }
        }
    }

    let password_hash = payload.password.map(|p| {
        let mut hasher = Sha256::new();
        hasher.update(p.as_bytes());
        hex::encode(hasher.finalize())
    });

    let user_record = UserRecord {
        id: user_id,
        email: email_copy,
        password_hash,
        created_at: Utc::now(),
    };

    {
        let mut users = state.users.write();
        users.insert(user_id, user_record);
    }

    // Generate tokens
    let access_token = format!("clipmesh_at_{}_{}", user_id, Uuid::new_v4());
    let refresh_token = format!("clipmesh_rt_{}_{}", user_id, Uuid::new_v4());

    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    let rt_hash = hex::encode(hasher.finalize());

    let expires_at = Utc::now() + Duration::days(30);
    {
        let mut refresh_sessions = state.refresh_sessions.write();
        refresh_sessions.insert(rt_hash, (user_id, expires_at));
    }

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user_id,
        expires_in: 900,
    }))
}

/// Login with email and password
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let users = state.users.read();
    let user = users
        .values()
        .find(|u| u.email.as_ref() == Some(&payload.email))
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    let mut hasher = Sha256::new();
    hasher.update(payload.password.as_bytes());
    let input_hash = hex::encode(hasher.finalize());

    if user.password_hash.as_ref() != Some(&input_hash) {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let user_id = user.id;
    let access_token = format!("clipmesh_at_{}_{}", user_id, Uuid::new_v4());
    let refresh_token = format!("clipmesh_rt_{}_{}", user_id, Uuid::new_v4());

    let mut rt_hasher = Sha256::new();
    rt_hasher.update(refresh_token.as_bytes());
    let rt_hash = hex::encode(rt_hasher.finalize());

    let expires_at = Utc::now() + Duration::days(30);
    {
        let mut refresh_sessions = state.refresh_sessions.write();
        refresh_sessions.insert(rt_hash, (user_id, expires_at));
    }

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user_id,
        expires_in: 900,
    }))
}

/// Refresh access token
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let mut hasher = Sha256::new();
    hasher.update(payload.refresh_token.as_bytes());
    let rt_hash = hex::encode(hasher.finalize());

    let user_id = {
        let mut refresh_sessions = state.refresh_sessions.write();
        let (uid, expires_at) = refresh_sessions
            .remove(&rt_hash)
            .ok_or_else(|| ApiError::Unauthorized("Invalid refresh token".to_string()))?;

        if Utc::now() > expires_at {
            return Err(ApiError::Unauthorized("Refresh token expired".to_string()));
        }
        uid
    };

    // Issue rotating new tokens
    let new_access_token = format!("clipmesh_at_{}_{}", user_id, Uuid::new_v4());
    let new_refresh_token = format!("clipmesh_rt_{}_{}", user_id, Uuid::new_v4());

    let mut new_hasher = Sha256::new();
    new_hasher.update(new_refresh_token.as_bytes());
    let new_rt_hash = hex::encode(new_hasher.finalize());

    let expires_at = Utc::now() + Duration::days(30);
    {
        let mut refresh_sessions = state.refresh_sessions.write();
        refresh_sessions.insert(new_rt_hash, (user_id, expires_at));
    }

    Ok(Json(AuthResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
        user_id,
        expires_in: 900,
    }))
}

/// Logout / revoke session
pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut hasher = Sha256::new();
    hasher.update(payload.refresh_token.as_bytes());
    let rt_hash = hex::encode(hasher.finalize());

    let mut refresh_sessions = state.refresh_sessions.write();
    refresh_sessions.remove(&rt_hash);

    Ok(Json(serde_json::json!({ "status": "logged_out" })))
}

