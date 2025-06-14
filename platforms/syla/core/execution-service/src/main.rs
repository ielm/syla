use anyhow::Result;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use redis::aio::ConnectionManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod docker;
mod error;
mod executor;
mod grpc;
mod models;
mod queue;
mod state;
mod worker;

use error::ServiceError;
use state::ServiceState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "syla_execution_service=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to Redis
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380/".to_string());
    let redis_client = redis::Client::open(redis_url)?;
    let redis_conn = ConnectionManager::new(redis_client).await?;

    // Initialize components
    let redis_queue = Arc::new(queue::RedisQueue::new(redis_conn.clone()));
    let docker_executor = Arc::new(executor::DockerExecutor::new().await?);

    // Initialize state for REST API
    let state = Arc::new(ServiceState {
        redis: Arc::new(Mutex::new(redis_conn)),
        docker_executor: Arc::new(docker::DockerExecutor::new()?),
    });

    // Start worker task
    let worker_state = state.clone();
    tokio::spawn(async move {
        worker::run_worker(worker_state).await;
    });

    // Start gRPC server
    let grpc_queue = redis_queue.clone();
    let grpc_executor = docker_executor.clone();
    tokio::spawn(async move {
        let addr = "0.0.0.0:8081".parse().unwrap();
        tracing::info!("Starting gRPC server on {}", addr);
        
        let service = grpc::server::ExecutionServiceImpl::new(grpc_queue, grpc_executor);
        
        tonic::transport::Server::builder()
            .add_service(grpc::proto::syla::execution::v1::execution_service_server::ExecutionServiceServer::new(service))
            .serve(addr)
            .await
            .expect("gRPC server failed");
    });

    // Build REST router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/executions", post(create_execution))
        .route("/executions/:id", get(get_execution))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start REST server
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8083".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting REST API on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn create_execution(
    State(state): State<Arc<ServiceState>>,
    Json(request): Json<models::CreateExecutionRequest>,
) -> Result<Json<models::ExecutionJob>, ServiceError> {
    let job = state.create_execution(request).await?;
    Ok(Json(job))
}

async fn get_execution(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::ExecutionJob>, ServiceError> {
    let job = state.get_execution(id).await?;
    Ok(Json(job))
}