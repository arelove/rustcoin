//! Axum REST-хендлеры.

use crate::server::AppState;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::Arc;

/// Returns a block by its hex-encoded hash.
pub async fn get_block_by_hash(
    Extension(state): Extension<Arc<AppState>>,
    Path(hash_hex): Path<String>,
) -> impl IntoResponse {
    let hash = match rc_primitives::hash::Hash::from_hex(&hash_hex) {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid hash"})),
            )
        }
    };

    match state.db.get_block(&hash) {
        Ok(Some(block)) => (StatusCode::OK, Json(json!(block))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "block not found"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Returns a block at the given height.
pub async fn get_block_by_height(
    Extension(state): Extension<Arc<AppState>>,
    Path(height): Path<u64>,
) -> impl IntoResponse {
    match state.db.get_block_at(height.into()) {
        Ok(Some(block)) => (StatusCode::OK, Json(json!(block))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "block not found"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Returns a transaction by its hex-encoded txid.
pub async fn get_transaction(
    Extension(state): Extension<Arc<AppState>>,
    Path(txid_hex): Path<String>,
) -> impl IntoResponse {
    let txid = match rc_primitives::hash::Hash::from_hex(&txid_hex) {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid txid"})),
            )
        }
    };
    match state.db.get_tx(&txid) {
        Ok(Some(tx)) => (StatusCode::OK, Json(json!(tx))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "tx not found"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Returns the account state for the given Base58 address.
pub async fn get_account(
    Extension(state): Extension<Arc<AppState>>,
    Path(addr_b58): Path<String>,
) -> impl IntoResponse {
    let addr = match rc_primitives::types::Address::from_base58(&addr_b58) {
        Ok(a) => a,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid address"})),
            )
        }
    };
    match state.db.get_account(&addr) {
        Ok(account) => (StatusCode::OK, Json(json!(account))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Returns current mempool size and total pending fees.
pub async fn get_mempool_info(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "size":        state.mempool.len(),
            "total_fees":  state.mempool.total_fees(),
        })),
    )
}
/// Returns basic chain information such as network name and version.
pub async fn get_chain_info(Extension(_state): Extension<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "network":    "quench-mainnet",
            "version":    env!("CARGO_PKG_VERSION"),
        })),
    )
}
