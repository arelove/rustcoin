//! HTTP + JSON-RPC сервер.

use crate::error::RpcError;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use rc_mempool::Mempool;
use rc_storage::Database;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

/// Конфигурация RPC сервера
#[derive(Debug, Clone)]
pub struct RpcServerConfig {
    /// The socket address the RPC server will bind to.
    pub bind_addr: SocketAddr,
    /// Включить CORS (нужно для веб-эксплорера)
    pub enable_cors: bool,
}

impl Default for RpcServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8545".parse().unwrap(),
            enable_cors: true,
        }
    }
}

/// Общий state, доступный всем handler'ам
#[derive(Clone)]
pub struct AppState {
    /// Handle to the blockchain database.
    pub db: Database,
    /// Shared reference to the transaction mempool.
    pub mempool: Arc<Mempool>,
}

/// RPC сервер
pub struct RpcServer {
    config: RpcServerConfig,
    state: AppState,
}

impl RpcServer {
    /// Creates a new `RpcServer` with the given config, database, and mempool.
    pub fn new(config: RpcServerConfig, db: Database, mempool: Arc<Mempool>) -> Self {
        Self {
            config,
            state: AppState { db, mempool },
        }
    }

    /// Запустить сервер (блокирует до завершения)
    pub async fn run(self) -> Result<(), RpcError> {
        let addr = self.config.bind_addr;

        let mut router = Router::new()
            // Healthcheck
            .route("/health", get(health_handler))
            // JSON-RPC endpoint
            .route("/", post(jsonrpc_handler))
            // REST endpoints (дополнительно к JSON-RPC, удобны для эксплорера)
            .route(
                "/api/v1/blocks/:hash",
                get(crate::handlers::get_block_by_hash),
            )
            .route(
                "/api/v1/blocks/height/:h",
                get(crate::handlers::get_block_by_height),
            )
            .route("/api/v1/tx/:txid", get(crate::handlers::get_transaction))
            .route("/api/v1/account/:addr", get(crate::handlers::get_account))
            .route("/api/v1/mempool", get(crate::handlers::get_mempool_info))
            .route("/api/v1/chain", get(crate::handlers::get_chain_info))
            .layer(Extension(Arc::new(self.state)))
            .layer(TraceLayer::new_for_http());

        if self.config.enable_cors {
            router = router.layer(CorsLayer::permissive());
        }

        info!("RPC server listening on http://{addr}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| RpcError::Bind(e.to_string()))?;

        axum::serve(listener, router)
            .await
            .map_err(|e| RpcError::Serve(e.to_string()))
    }
}

/// GET /health
async fn health_handler() -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// POST / — JSON-RPC 2.0 диспетчер
async fn jsonrpc_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!([]));
    let id = req.get("id").cloned().unwrap_or(json!(null));

    let result = dispatch_rpc(state, method, params).await;

    let response = match result {
        Ok(data) => json!({
            "jsonrpc": "2.0",
            "result": data,
            "id": id,
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "error": { "code": e.code(), "message": e.to_string() },
            "id": id,
        }),
    };

    (StatusCode::OK, Json(response))
}

/// Маршрутизация JSON-RPC методов
async fn dispatch_rpc(
    state: Arc<AppState>,
    method: &str,
    params: Value,
) -> Result<Value, RpcError> {
    match method {
        "chain_getInfo" => {
            // Пример реализации
            Ok(json!({
                "network_name": "quench-mainnet",
                "best_height":  0,
                "is_syncing":   false,
            }))
        }
        "mempool_getSize" => Ok(json!({ "size": state.mempool.len() })),
        "tx_send" => {
            let raw_hex = params
                .get(0)
                .and_then(|v| v.as_str())
                .ok_or(RpcError::InvalidParams("expected hex string".into()))?;

            // Декодируем транзакцию
            let bytes =
                hex::decode(raw_hex).map_err(|_| RpcError::InvalidParams("invalid hex".into()))?;
            let tx: rc_primitives::transaction::Transaction = serde_json::from_slice(&bytes)
                .map_err(|e| RpcError::InvalidParams(e.to_string()))?;

            let txid = state
                .mempool
                .add(tx)
                .map_err(|e| RpcError::Internal(e.to_string()))?;

            Ok(json!({ "txid": txid.to_hex() }))
        }
        other => Err(RpcError::MethodNotFound(other.to_string())),
    }
}
