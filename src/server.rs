use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};

use crate::logic::SyndicaAppLogic;

pub async fn is_slot_confirmed(
    Path(slot): Path<u64>,
    State(logic): State<Arc<SyndicaAppLogic>>,
) -> Result<StatusCode, StatusCode> {
    let start_time = Instant::now();
    debug!(slot, "Checking if slot is confirmed");

    let result = match logic.get_block(slot).await {
        Ok(Some(_)) => {
            debug!(slot, "Slot {} confirmed", slot);
            Ok(StatusCode::OK)
        }
        Ok(None) => {
            debug!(slot, "Slot {} not confirmed", slot);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!(slot, error = %e, "Failed to check slot {}", slot);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    let elapsed = start_time.elapsed();
    logic
        .state()
        .metrics()
        .record_is_slot_confirmed_elapsed(elapsed);

    debug!(
        slot,
        elapsed_ms = elapsed.as_millis(),
        "Slot confirmation check completed"
    );

    result
}

pub fn create_router(logic: Arc<SyndicaAppLogic>) -> Router {
    Router::new()
        .route("/isSlotConfirmed/{slot}", get(is_slot_confirmed))
        .with_state(logic)
}

pub async fn start_server(
    port: u16,
    logic: Arc<SyndicaAppLogic>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(logic);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!(port, "Server starting");

    axum::serve(listener, app).await?;

    Ok(())
}
