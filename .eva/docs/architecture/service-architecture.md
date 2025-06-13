# Syla Service Architecture

## Overview

Syla adopts a polyrepo microservices architecture where each service is independently developed, deployed, and maintained. This document details the service structure, communication patterns, and implementation guidelines.

## Service Registry

### Core Services

| Service | Repository | Port | Purpose |
|---------|------------|------|---------|
| syla-api-gateway | `datacurve/syla-api-gateway` | 8080 | API entry point, routing, auth |
| syla-workspace-service | `datacurve/syla-workspace-service` | 8081 | Workspace lifecycle management |
| syla-execution-service | `datacurve/syla-execution-service` | 8082 | Execution orchestration |
| syla-sandbox-runtime | `datacurve/syla-sandbox-runtime` | 8083 | Sandbox management |
| syla-telemetry-service | `datacurve/syla-telemetry-service` | 8084 | Metrics and monitoring |
| syla-auth-service | `datacurve/syla-auth-service` | 8085 | Authentication/authorization |
| syla-scheduler-service | `datacurve/syla-scheduler-service` | 8086 | Job scheduling |
| syla-storage-service | `datacurve/syla-storage-service` | 8087 | File/artifact storage |

### Supporting Services

| Service | Repository | Port | Purpose |
|---------|------------|------|---------|
| syla-proto | `datacurve/syla-proto` | - | Shared protobuf definitions |
| syla-sdk-generator | `datacurve/syla-sdk-generator` | - | SDK generation tooling |
| syla-cli | `datacurve/syla-cli` | - | Developer CLI tool |
| syla-infra | `datacurve/syla-infra` | - | Infrastructure as code |

## Service Communication Architecture

```
                          ┌─────────────────┐
                          │   API Gateway   │
                          │   (gRPC/REST)   │
                          └────────┬────────┘
                                   │
                ┌──────────────────┼──────────────────┐
                │                  │                  │
       ┌────────▼────────┐ ┌──────▼──────┐ ┌────────▼────────┐
       │ Auth Service    │ │  Workspace  │ │   Execution     │
       │ (gRPC)         │ │  Service    │ │   Service       │
       └────────┬────────┘ │  (gRPC)     │ │   (gRPC)        │
                │          └──────┬──────┘ └────────┬────────┘
                │                 │                  │
                │          ┌──────▼──────┐          │
                └──────────► Storage     ◄──────────┘
                          │  Service    │
                          │  (gRPC)     │
                          └─────────────┘
```

## Service Templates

### Standard Service Structure

```
syla-<service-name>/
├── src/
│   ├── main.rs              # Service entry point
│   ├── server.rs            # gRPC server implementation
│   ├── handlers/            # Request handlers
│   ├── models/              # Domain models
│   ├── repository/          # Data access layer
│   ├── services/            # Business logic
│   ├── config.rs            # Configuration
│   └── errors.rs            # Error types
├── proto/
│   └── service.proto        # Service-specific protos
├── migrations/              # Database migrations
├── tests/
│   ├── integration/         # Integration tests
│   └── unit/               # Unit tests
├── Cargo.toml
├── Dockerfile
├── Makefile
├── README.md
└── .github/
    └── workflows/
        ├── ci.yml          # CI pipeline
        └── release.yml     # Release pipeline
```

### Service Implementation Pattern

```rust
// src/main.rs
use anyhow::Result;
use syla_common::{config::Config, telemetry::init_telemetry};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration
    let config = Config::from_env()?;
    
    // Initialize telemetry
    init_telemetry(&config.service_name)?;
    
    // Initialize database
    let db_pool = init_database(&config.database_url).await?;
    
    // Create service
    let service = MyService::new(config, db_pool);
    
    // Start gRPC server
    let server = Server::builder()
        .add_service(MyServiceServer::new(service))
        .serve(config.listen_addr);
    
    // Graceful shutdown
    tokio::select! {
        _ = server => {},
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down gracefully");
        }
    }
    
    Ok(())
}
```

## Service Specifications

### 1. API Gateway Service

**Purpose**: Single entry point for all client requests

```rust
pub struct ApiGateway {
    auth_client: AuthServiceClient,
    workspace_client: WorkspaceServiceClient,
    execution_client: ExecutionServiceClient,
    rate_limiter: RateLimiter,
    circuit_breaker: CircuitBreaker,
}

// Key responsibilities:
// - Request routing
// - Authentication/authorization
// - Rate limiting
// - Circuit breaking
// - Request/response transformation
// - API versioning
```

**Key Features**:
- GraphQL and REST API support
- WebSocket for real-time updates
- Request validation and sanitization
- Response caching
- API documentation generation

### 2. Workspace Service

**Purpose**: Manage workspace lifecycle and state

```rust
pub struct WorkspaceService {
    repository: WorkspaceRepository,
    storage_client: StorageServiceClient,
    event_publisher: EventPublisher,
    state_manager: StateManager,
}

// Key APIs:
trait WorkspaceService {
    async fn create_workspace(&self, req: CreateWorkspaceRequest) -> Result<Workspace>;
    async fn get_workspace(&self, id: Uuid) -> Result<Workspace>;
    async fn update_workspace(&self, id: Uuid, req: UpdateRequest) -> Result<Workspace>;
    async fn delete_workspace(&self, id: Uuid) -> Result<()>;
    async fn list_workspaces(&self, filter: WorkspaceFilter) -> Result<Vec<Workspace>>;
    async fn share_workspace(&self, id: Uuid, req: ShareRequest) -> Result<()>;
}
```

**Database Schema**:
```sql
CREATE TABLE workspaces (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    owner_id VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    
    INDEX idx_owner (owner_id, created_at),
    INDEX idx_type_status (type, status),
    INDEX idx_expires (expires_at) WHERE expires_at IS NOT NULL
);

CREATE TABLE workspace_shares (
    workspace_id UUID REFERENCES workspaces(id),
    user_id VARCHAR(255) NOT NULL,
    permission VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    
    PRIMARY KEY (workspace_id, user_id)
);
```

### 3. Execution Service

**Purpose**: Orchestrate code execution across sandbox fleet

```rust
pub struct ExecutionService {
    scheduler: ExecutionScheduler,
    sandbox_client: SandboxRuntimeClient,
    workspace_client: WorkspaceServiceClient,
    telemetry_client: TelemetryServiceClient,
    execution_cache: ExecutionCache,
}

// Core execution flow:
async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult> {
    // 1. Validate request
    self.validate_request(&request)?;
    
    // 2. Check cache
    if let Some(cached) = self.execution_cache.get(&request).await? {
        return Ok(cached);
    }
    
    // 3. Schedule execution
    let placement = self.scheduler.schedule(&request).await?;
    
    // 4. Execute in sandbox
    let result = self.sandbox_client
        .execute_at(placement, request)
        .await?;
    
    // 5. Record telemetry
    self.telemetry_client.record_execution(&result).await?;
    
    // 6. Cache result
    self.execution_cache.put(&request, &result).await?;
    
    Ok(result)
}
```

**Scheduling Algorithm**:
```rust
pub struct IntelligentScheduler {
    nodes: Vec<ExecutionNode>,
    predictor: LoadPredictor,
    optimizer: PlacementOptimizer,
}

impl IntelligentScheduler {
    async fn schedule(&self, request: &ExecutionRequest) -> Result<Placement> {
        // Get current system state
        let system_state = self.get_system_state().await?;
        
        // Predict resource requirements
        let requirements = self.predictor.predict(request)?;
        
        // Find optimal placement
        let placement = self.optimizer.optimize(
            &requirements,
            &system_state,
            &self.get_constraints(request),
        )?;
        
        Ok(placement)
    }
}
```

### 4. Sandbox Runtime Service

**Purpose**: Manage Firecracker microVM fleet and execute code

```rust
pub struct SandboxRuntime {
    vm_pool: VmPool,
    supervisor: SandboxSupervisor,
    security_manager: SecurityManager,
    resource_monitor: ResourceMonitor,
}

// VM lifecycle management
impl SandboxRuntime {
    async fn acquire_vm(&self, language: Language) -> Result<VmHandle> {
        // Try to get warm VM from pool
        if let Some(vm) = self.vm_pool.get_warm(language).await {
            return Ok(vm);
        }
        
        // Create new VM if needed
        self.create_vm(language).await
    }
    
    async fn execute_in_vm(
        &self,
        vm: VmHandle,
        request: ExecutionRequest,
    ) -> Result<ExecutionResult> {
        // Apply security policies
        self.security_manager.apply(&vm, &request)?;
        
        // Execute with monitoring
        let result = self.supervisor
            .execute(&vm, request)
            .with_monitoring(&self.resource_monitor)
            .await?;
        
        // Return VM to pool or destroy
        if result.is_clean() {
            self.vm_pool.return_vm(vm).await;
        } else {
            self.destroy_vm(vm).await;
        }
        
        Ok(result)
    }
}
```

**VM Pool Configuration**:
```yaml
vm_pool:
  warm_pool_size: 200
  max_vms: 1000
  vm_ttl: 300s
  
  prewarming:
    enabled: true
    prediction_window: 15m
    scale_factor: 1.3
    
  resources:
    cpu_per_vm: 2
    memory_per_vm: 2048
    disk_per_vm: 10240
```

### 5. Telemetry Service

**Purpose**: Collect, process, and export execution telemetry

```rust
pub struct TelemetryService {
    event_ingester: EventIngester,
    metrics_processor: MetricsProcessor,
    trace_aggregator: TraceAggregator,
    export_pipeline: ExportPipeline,
}

// Telemetry processing pipeline
impl TelemetryService {
    async fn ingest_event(&self, event: TelemetryEvent) -> Result<()> {
        // Validate and enrich event
        let enriched = self.enrich_event(event).await?;
        
        // Process based on event type
        match enriched {
            TelemetryEvent::Execution(e) => {
                self.metrics_processor.process_execution(&e).await?;
                self.trace_aggregator.add_execution_trace(&e).await?;
            }
            TelemetryEvent::System(e) => {
                self.metrics_processor.process_system(&e).await?;
            }
            // ... other event types
        }
        
        // Export for training data
        if enriched.is_training_eligible() {
            self.export_pipeline.export(&enriched).await?;
        }
        
        Ok(())
    }
}
```

**Metrics Schema**:
```rust
pub struct ExecutionMetrics {
    // Identifiers
    pub execution_id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: String,
    
    // Timing
    pub queue_duration_ms: u64,
    pub startup_duration_ms: u64,
    pub execution_duration_ms: u64,
    pub total_duration_ms: u64,
    
    // Resources
    pub cpu_usage_millicores: u64,
    pub memory_peak_mb: u64,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
    
    // Outcomes
    pub success: bool,
    pub exit_code: i32,
    pub error_type: Option<String>,
}
```

### 6. Auth Service

**Purpose**: Handle authentication and authorization

```rust
pub struct AuthService {
    user_repository: UserRepository,
    token_manager: TokenManager,
    permission_manager: PermissionManager,
    audit_logger: AuditLogger,
}

// Authentication flow
impl AuthService {
    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse> {
        let user = match req {
            AuthRequest::Password { email, password } => {
                self.authenticate_password(email, password).await?
            }
            AuthRequest::OAuth { provider, token } => {
                self.authenticate_oauth(provider, token).await?
            }
            AuthRequest::ApiKey { key } => {
                self.authenticate_api_key(key).await?
            }
        };
        
        // Generate tokens
        let access_token = self.token_manager.create_access_token(&user)?;
        let refresh_token = self.token_manager.create_refresh_token(&user)?;
        
        // Audit log
        self.audit_logger.log_authentication(&user).await?;
        
        Ok(AuthResponse {
            access_token,
            refresh_token,
            user,
        })
    }
}
```

**Permission Model**:
```rust
pub enum Permission {
    // Workspace permissions
    WorkspaceCreate,
    WorkspaceRead(Uuid),
    WorkspaceWrite(Uuid),
    WorkspaceDelete(Uuid),
    WorkspaceShare(Uuid),
    
    // Execution permissions
    ExecutionCreate,
    ExecutionRead(Uuid),
    ExecutionCancel(Uuid),
    
    // Admin permissions
    AdminUserManage,
    AdminSystemConfig,
    AdminMetricsView,
}
```

## Inter-Service Communication

### gRPC Service Definitions

```protobuf
// proto/workspace.proto
syntax = "proto3";

package syla.workspace.v1;

service WorkspaceService {
    rpc CreateWorkspace(CreateWorkspaceRequest) returns (Workspace);
    rpc GetWorkspace(GetWorkspaceRequest) returns (Workspace);
    rpc UpdateWorkspace(UpdateWorkspaceRequest) returns (Workspace);
    rpc DeleteWorkspace(DeleteWorkspaceRequest) returns (Empty);
    rpc ListWorkspaces(ListWorkspacesRequest) returns (ListWorkspacesResponse);
    rpc ShareWorkspace(ShareWorkspaceRequest) returns (Empty);
}

message Workspace {
    string id = 1;
    string name = 2;
    string owner_id = 3;
    WorkspaceType type = 4;
    WorkspaceStatus status = 5;
    google.protobuf.Timestamp created_at = 6;
    google.protobuf.Timestamp expires_at = 7;
    map<string, string> metadata = 8;
}
```

### Service Client Pattern

```rust
// Shared client wrapper with circuit breaking and retries
pub struct ServiceClient<T> {
    inner: T,
    circuit_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
}

impl<T> ServiceClient<T> {
    pub async fn call<F, R>(&self, f: F) -> Result<R>
    where
        F: Fn(&T) -> BoxFuture<'_, Result<R>>,
    {
        self.circuit_breaker
            .call(|| {
                self.retry_policy.retry(|| f(&self.inner))
            })
            .await
    }
}

// Usage example
let workspace = workspace_client
    .call(|client| {
        Box::pin(client.get_workspace(GetWorkspaceRequest { id }))
    })
    .await?;
```

## Service Discovery

### Configuration-Based Discovery

```toml
# services.toml
[services]
api_gateway = { url = "http://api-gateway:8080" }
workspace_service = { url = "http://workspace-service:8081" }
execution_service = { url = "http://execution-service:8082" }
sandbox_runtime = { url = "http://sandbox-runtime:8083" }
telemetry_service = { url = "http://telemetry-service:8084" }
auth_service = { url = "http://auth-service:8085" }

[environment.production]
api_gateway = { url = "https://api.syla.dev" }
workspace_service = { url = "workspace.internal.syla.dev:443" }
# ... production URLs
```

### Health Checks

```rust
// Standard health check implementation
#[tonic::async_trait]
impl Health for MyService {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        // Check dependencies
        let db_healthy = self.check_database().await;
        let deps_healthy = self.check_dependencies().await;
        
        let status = if db_healthy && deps_healthy {
            ServingStatus::Serving
        } else {
            ServingStatus::NotServing
        };
        
        Ok(Response::new(HealthCheckResponse {
            status: status as i32,
        }))
    }
}
```

## Database Per Service

Each service maintains its own database:

| Service | Database | Type | Purpose |
|---------|----------|------|---------|
| workspace-service | `syla_workspaces` | PostgreSQL | Workspace metadata |
| execution-service | `syla_executions` | PostgreSQL | Execution history |
| auth-service | `syla_auth` | PostgreSQL | Users, permissions |
| telemetry-service | `syla_telemetry` | TimescaleDB | Time-series metrics |
| storage-service | `syla_storage` | PostgreSQL + S3 | File metadata |

## Service Deployment

### Kubernetes Manifests

```yaml
# k8s/workspace-service.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: workspace-service
  namespace: syla
spec:
  replicas: 3
  selector:
    matchLabels:
      app: workspace-service
  template:
    metadata:
      labels:
        app: workspace-service
    spec:
      containers:
      - name: workspace-service
        image: datacurve/syla-workspace-service:v1.0.0
        ports:
        - containerPort: 8081
          name: grpc
        - containerPort: 9090
          name: metrics
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: workspace-service-secrets
              key: database-url
        - name: RUST_LOG
          value: info
        livenessProbe:
          grpc:
            port: 8081
        readinessProbe:
          grpc:
            port: 8081
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2
            memory: 2Gi
---
apiVersion: v1
kind: Service
metadata:
  name: workspace-service
  namespace: syla
spec:
  selector:
    app: workspace-service
  ports:
  - port: 8081
    targetPort: 8081
    name: grpc
  - port: 9090
    targetPort: 9090
    name: metrics
```

### Service Mesh Integration

```yaml
# Istio VirtualService
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: workspace-service
  namespace: syla
spec:
  hosts:
  - workspace-service
  http:
  - timeout: 30s
    retries:
      attempts: 3
      perTryTimeout: 10s
      retryOn: 5xx,deadline-exceeded
    route:
    - destination:
        host: workspace-service
        port:
          number: 8081
```

## Service Testing

### Integration Testing Pattern

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use testcontainers::{clients, images::postgres::Postgres};
    
    #[tokio::test]
    async fn test_create_workspace() {
        // Start test containers
        let docker = clients::Cli::default();
        let postgres = docker.run(Postgres::default());
        
        // Initialize service
        let service = WorkspaceService::new(
            &postgres.get_connection_string(),
        ).await.unwrap();
        
        // Test workspace creation
        let request = CreateWorkspaceRequest {
            name: "test-workspace".to_string(),
            workspace_type: WorkspaceType::Ephemeral,
        };
        
        let workspace = service.create_workspace(request).await.unwrap();
        
        assert_eq!(workspace.name, "test-workspace");
        assert_eq!(workspace.status, WorkspaceStatus::Active);
    }
}
```

### Contract Testing

```rust
// Pact contract tests
#[cfg(test)]
mod contract_tests {
    use pact_consumer::prelude::*;
    
    #[tokio::test]
    async fn test_execution_service_contract() {
        let pact = PactBuilder::new("workspace-service", "execution-service")
            .interaction("create execution", |i| {
                i.given("workspace exists")
                    .request("POST", "/executions")
                    .json_body(json!({
                        "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
                        "language": "python",
                        "code": "print('hello')"
                    }))
                    .response(201)
                    .json_body(json!({
                        "id": like!("550e8400-e29b-41d4-a716-446655440000"),
                        "status": "pending"
                    }))
            })
            .build();
            
        // Test implementation
        pact.verify().await;
    }
}
```

## Service Monitoring

### Metrics Collection

```rust
use prometheus::{Counter, Histogram, Registry};

pub struct ServiceMetrics {
    requests_total: Counter,
    request_duration: Histogram,
    errors_total: Counter,
}

impl ServiceMetrics {
    pub fn new(registry: &Registry) -> Self {
        let requests_total = Counter::new(
            "grpc_requests_total",
            "Total number of gRPC requests"
        ).unwrap();
        
        let request_duration = Histogram::new(
            "grpc_request_duration_seconds",
            "gRPC request duration"
        ).unwrap();
        
        let errors_total = Counter::new(
            "grpc_errors_total",
            "Total number of gRPC errors"
        ).unwrap();
        
        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();
        registry.register(Box::new(errors_total.clone())).unwrap();
        
        Self {
            requests_total,
            request_duration,
            errors_total,
        }
    }
}
```

### Distributed Tracing

```rust
use opentelemetry::{global, trace::Tracer};
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn init_tracing(service_name: &str) {
    let tracer = global::tracer(service_name);
    
    let telemetry_layer = OpenTelemetryLayer::new(tracer);
    
    tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// Usage in handlers
#[tracing::instrument(skip(self))]
pub async fn create_workspace(
    &self,
    request: CreateWorkspaceRequest,
) -> Result<Workspace> {
    tracing::info!("Creating workspace");
    
    let workspace = self.repository
        .create(request)
        .instrument(tracing::info_span!("db_create"))
        .await?;
        
    tracing::info!(workspace_id = %workspace.id, "Workspace created");
    
    Ok(workspace)
}
```

## Service Evolution

### API Versioning Strategy

```protobuf
// Support multiple API versions
package syla.workspace.v2;

service WorkspaceService {
    // v2 adds batch operations
    rpc BatchCreateWorkspaces(BatchCreateRequest) returns (BatchCreateResponse);
    
    // Existing v1 methods still supported
    rpc CreateWorkspace(CreateWorkspaceRequest) returns (Workspace);
}
```

### Database Migration Pattern

```sql
-- migrations/001_initial_schema.sql
CREATE TABLE workspaces (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

-- migrations/002_add_workspace_type.sql
ALTER TABLE workspaces
ADD COLUMN type VARCHAR(50) NOT NULL DEFAULT 'ephemeral';

-- migrations/003_add_expiry.sql
ALTER TABLE workspaces
ADD COLUMN expires_at TIMESTAMPTZ;

CREATE INDEX idx_expires ON workspaces(expires_at)
WHERE expires_at IS NOT NULL;
```

## Development Workflow

### Local Development

```bash
# Start all services locally
syla dev up

# Start specific services
syla dev up workspace-service execution-service

# View logs
syla dev logs workspace-service -f

# Run tests
syla dev test workspace-service

# Debug a service
syla dev debug workspace-service
```

### CI/CD Pipeline

```yaml
# .github/workflows/service-ci.yml
name: Service CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    
    - name: Run tests
      run: |
        cargo test --all-features
        cargo test --doc
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Build Docker image
      run: docker build -t $SERVICE_NAME:$GITHUB_SHA .
    
    - name: Run integration tests
      run: |
        docker-compose -f docker-compose.test.yml up -d
        cargo test --test integration
        docker-compose -f docker-compose.test.yml down
```

## Best Practices

1. **Service Independence**: Each service should be deployable independently
2. **API First**: Design APIs before implementation
3. **Backward Compatibility**: Never break existing APIs
4. **Comprehensive Testing**: Unit, integration, and contract tests
5. **Observability**: Metrics, logs, and traces from day one
6. **Security**: mTLS between services, principle of least privilege
7. **Documentation**: Keep service docs up to date
8. **Error Handling**: Use consistent error types across services
9. **Configuration**: Environment-based configuration
10. **Graceful Degradation**: Services should handle dependency failures