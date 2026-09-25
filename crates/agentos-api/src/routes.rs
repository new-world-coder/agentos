use agentos_core::{Run, RunId};
use agentos_runtime::{ApprovalDecision, StepOutcome};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/runs", post(create_run).get(list_runs))
        .route("/v1/runs/{id}", get(get_run))
        .route("/v1/runs/{id}/drive", post(drive_run))
        .route("/v1/runs/{id}/resume", post(resume_run))
        .route("/v1/runs/{id}/approve", post(approve))
        .route("/v1/runs/{id}/journal", get(get_journal))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "agentos",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Deserialize)]
struct CreateRunRequest {
    goal: String,
}

#[derive(Serialize)]
struct CreateRunResponse {
    run: Run,
}

async fn create_run(
    State(state): State<AppState>,
    Json(body): Json<CreateRunRequest>,
) -> Result<Json<CreateRunResponse>, ApiError> {
    let run = state.runtime.create_run(body.goal).await?;
    Ok(Json(CreateRunResponse { run }))
}

async fn list_runs(State(state): State<AppState>) -> Result<Json<Vec<Run>>, ApiError> {
    let runs = state.runtime.store().list_runs().await?;
    Ok(Json(runs))
}

async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Run>, ApiError> {
    let run = state
        .runtime
        .store()
        .get_run(&RunId(id.clone()))
        .await?
        .ok_or_else(|| ApiError::not_found(format!("run {id} not found")))?;
    Ok(Json(run))
}

#[derive(Serialize)]
struct DriveResponse {
    outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    effect_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    run: Run,
}

fn outcome_response(outcome: StepOutcome, run: Run) -> DriveResponse {
    match outcome {
        StepOutcome::Completed => DriveResponse {
            outcome: "completed".into(),
            effect_id: None,
            error: None,
            run,
        },
        StepOutcome::AwaitingApproval { effect_id } => DriveResponse {
            outcome: "awaiting_approval".into(),
            effect_id: Some(effect_id),
            error: None,
            run,
        },
        StepOutcome::Failed { error } => DriveResponse {
            outcome: "failed".into(),
            effect_id: None,
            error: Some(error),
            run,
        },
    }
}

async fn drive_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DriveResponse>, ApiError> {
    let run_id = RunId(id);
    let outcome = state.runtime.drive(&run_id).await?;
    let run = state
        .runtime
        .store()
        .get_run(&run_id)
        .await?
        .ok_or_else(|| ApiError::not_found("run disappeared".into()))?;
    Ok(Json(outcome_response(outcome, run)))
}

async fn resume_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DriveResponse>, ApiError> {
    let run_id = RunId(id);
    let outcome = state.runtime.resume(&run_id).await?;
    let run = state
        .runtime
        .store()
        .get_run(&run_id)
        .await?
        .ok_or_else(|| ApiError::not_found("run disappeared".into()))?;
    Ok(Json(outcome_response(outcome, run)))
}

#[derive(Deserialize)]
struct ApproveRequest {
    effect_id: String,
    #[serde(default = "default_approve")]
    approve: bool,
    #[serde(default)]
    reason: Option<String>,
}

fn default_approve() -> bool {
    true
}

async fn approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ApproveRequest>,
) -> Result<Json<DriveResponse>, ApiError> {
    let run_id = RunId(id);
    let decision = if body.approve {
        ApprovalDecision::Approve
    } else {
        ApprovalDecision::Reject {
            reason: body.reason.unwrap_or_else(|| "rejected".into()),
        }
    };
    let outcome = state
        .runtime
        .resolve_approval(&run_id, &body.effect_id, decision)
        .await?;
    let run = state
        .runtime
        .store()
        .get_run(&run_id)
        .await?
        .ok_or_else(|| ApiError::not_found("run disappeared".into()))?;
    Ok(Json(outcome_response(outcome, run)))
}

async fn get_journal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entries = state.runtime.store().list_journal(&RunId(id)).await?;
    Ok(Json(serde_json::json!({ "entries": entries })))
}

pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn not_found(message: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message,
        }
    }
}

impl From<agentos_core::CoreError> for ApiError {
    fn from(value: agentos_core::CoreError) -> Self {
        let status = match &value {
            agentos_core::CoreError::RunNotFound(_)
            | agentos_core::CoreError::EffectNotFound(_) => StatusCode::NOT_FOUND,
            agentos_core::CoreError::PolicyDenied(_) => StatusCode::FORBIDDEN,
            agentos_core::CoreError::AwaitingApproval(_) => StatusCode::CONFLICT,
            _ => StatusCode::BAD_REQUEST,
        };
        Self {
            status,
            message: value.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}
