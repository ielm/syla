# Shipd Code Execution Platform: World-Class Architecture & Implementation Guide

## Executive Summary

Shipd needs a **code execution platform** that can securely run developer solutions at scale while collecting comprehensive execution data for LLM training. This document presents a production-ready architecture that delivers:

- **Sub-200ms cold starts** for code execution with hardware-level isolation
- **10,000+ concurrent executions** per host with minimal overhead
- **Multi-language support** (Python, JavaScript, Go, Rust, Java, C++, and more)
- **Comprehensive telemetry** for training data (execution traces, performance metrics, resource usage)
- **Enterprise-grade security** with defense-in-depth isolation
- **Global scale** with multi-region deployment and intelligent routing
- **99.99% availability** through redundancy and automatic failover

## System Architecture

### Core Design Principles

1. **Execution-First Design**: Every architectural decision optimizes for fast, secure code execution
2. **Data Collection Excellence**: Rich telemetry collection without impacting performance
3. **Language Agnostic**: Extensible architecture supporting any programming language
4. **Scale Without Limits**: Horizontal scaling to millions of executions per day
5. **Security by Default**: Multiple layers of isolation protecting against malicious code

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Shipd Platform                                │
│           (Next.js Frontend + Elixir/Phoenix Backend)                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ gRPC/REST
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Code Execution Gateway                           │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐      │
│  │ Request Router  │  │ Auth & Rate Limit│  │ Telemetry Collector│      │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                          ┌─────────┴─────────┐
                          │                   │
                    ┌─────▼─────┐       ┌─────▼─────┐
                    │ Region 1  │       │ Region 2  │
                    └───────────┘       └───────────┘
                          │
┌─────────────────────────────────────────────────────────────────────────┐
│                        Execution Orchestrator                           │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐      │
│  │ Scheduler       │  │ Resource Manager │  │ Execution Tracker  │      │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                         Sandbox Runtime Layer                           │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐      │
│  │ VM Pool Manager │  │ Workspace Manager│  │ Security Enforcer  │      │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│              Firecracker MicroVM Fleet (Universal VMs)                  │
│  ┌───────────────────────────────────────────────────────────────┐      │
│  │  Each VM Contains:                                            │      │
│  │  • Single Rust Supervisor (all languages)                     │      │
│  │  • Python, Node.js, Go, Rust, Java, C++ Runtimes              │      │
│  │  • Workspace Management                                       │      │
│  │  • Security Enforcement                                       │      │
│  │  • Telemetry Collection                                       │      │
│  └───────────────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Code Execution Gateway

The gateway serves as the single entry point for all code execution requests from the Shipd platform.

```rust
// Gateway API Design - Supporting both simple and complex executions
pub struct ExecutionRequest {
    pub id: Uuid,
    pub user_id: String,
    pub problem_id: String,
    pub language: Language,
    pub source: ExecutionSource,
    pub entry_point: String,        // e.g., "main.py", "npm start", "cargo run"
    pub arguments: Vec<String>,     // Command line arguments
    pub test_cases: Vec<TestCase>,
    pub constraints: ExecutionConstraints,
    pub outputs: Vec<String>,       // Files to collect after execution
    pub metadata: HashMap<String, Value>,
}

pub enum ExecutionSource {
    // Simple single-file execution
    Code {
        filename: String,
        content: String,
    },

    // Multi-file project
    Files(Vec<FileEntry>),

    // Git repository
    GitRepo {
        url: String,
        branch: String,
        commit: Option<String>,
    },

    // Compressed archive
    Archive {
        data: Vec<u8>,
        format: ArchiveFormat,
    },
}

pub struct FileEntry {
    pub path: String,
    pub content: String,
    pub mode: u32,  // Unix file permissions
}

pub struct ExecutionConstraints {
    pub timeout_ms: u32,        // Max 300000 (5 minutes)
    pub memory_mb: u32,         // Max 8192 (8GB)
    pub cpu_millicores: u32,    // Max 4000 (4 cores)
    pub disk_mb: u32,           // Max 10240 (10GB)
    pub network_enabled: bool,  // Default: false
    pub network_rules: Option<NetworkRules>,
}

pub struct TestCase {
    pub name: String,
    pub input: TestInput,
    pub expected_output: TestOutput,
    pub timeout_ms: Option<u32>,
    pub is_hidden: bool,
}

pub enum TestInput {
    Stdin(String),
    Files(Vec<FileEntry>),
    Arguments(Vec<String>),
    Combined {
        stdin: Option<String>,
        files: Vec<FileEntry>,
        args: Vec<String>,
    },
}

pub enum TestOutput {
    Stdout(String),
    Files(HashMap<String, String>),
    ExitCode(i32),
    Combined {
        stdout: Option<String>,
        files: HashMap<String, String>,
        exit_code: i32,
    },
}
```

#### Key Features

- **Smart Routing**: Routes executions to optimal regions based on load and latency
- **Request Validation**: Ensures code and test cases meet security requirements
- **Rate Limiting**: Per-user and global rate limits to prevent abuse
- **Telemetry Injection**: Adds tracking headers for distributed tracing

### 2. Execution Orchestrator

The orchestrator manages the lifecycle of code executions across the sandbox fleet.

```rust
pub struct ExecutionOrchestrator {
    scheduler: Arc<Scheduler>,
    resource_manager: Arc<ResourceManager>,
    vm_pool: Arc<VmPoolManager>,
    telemetry: Arc<TelemetryCollector>,
}

impl ExecutionOrchestrator {
    pub async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult> {
        // 1. Schedule execution based on current load
        let placement = self.scheduler.find_optimal_placement(&request).await?;

        // 2. Reserve resources
        let resources = self.resource_manager
            .reserve(request.constraints, placement.node_id)
            .await?;

        // 3. Acquire VM from pool
        let vm = self.vm_pool
            .acquire(request.language, placement.node_id)
            .await?;

        // 4. Execute with telemetry
        let result = self.execute_with_telemetry(vm, request, resources).await?;

        // 5. Collect comprehensive metrics
        self.telemetry.record_execution(&result).await?;

        Ok(result)
    }
}
```

#### Scheduling Algorithm

```rust
pub struct Scheduler {
    nodes: Vec<ExecutionNode>,
    load_balancer: LoadBalancer,
}

impl Scheduler {
    pub async fn find_optimal_placement(&self, req: &ExecutionRequest) -> Result<Placement> {
        // Consider multiple factors:
        // 1. Current node load (CPU, memory, active VMs)
        // 2. Language-specific VM availability
        // 3. Geographic proximity to user
        // 4. Recent failure rates
        // 5. Cost optimization (spot vs on-demand)

        let candidates = self.nodes.iter()
            .filter(|n| n.can_handle(req))
            .map(|n| (n, self.score_node(n, req)))
            .collect::<Vec<_>>();

        candidates.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(node, _)| Placement { node_id: node.id })
            .ok_or_else(|| Error::NoAvailableNodes)
    }
}
```

### 3. Sandbox Runtime Layer

The runtime layer provides secure, isolated execution environments using Firecracker microVMs with a single Rust supervisor.

#### VM Pool Management (Universal VMs)

```rust
pub struct VmPool {
    warm_vms: VecDeque<WarmVm>,  // Single pool of universal VMs
    config: VmPoolConfig,
    metrics: PoolMetrics,
}

pub struct VmPoolConfig {
    pub target_warm_vms: usize,     // Total warm VMs (not per-language)
    pub max_vms: usize,             // Maximum concurrent VMs
    pub vm_ttl: Duration,           // Time before recycling
    pub preemptive_scaling: bool,   // Auto-scale based on load
}

impl Default for VmPoolConfig {
    fn default() -> Self {
        Self {
            target_warm_vms: 200,       // Universal VMs ready for any language
            max_vms: 1000,              // Maximum concurrent executions
            vm_ttl: Duration::from_secs(300), // 5 minutes
            preemptive_scaling: true,
        }
    }
}
```

#### Firecracker VM Configuration

```json
{
  "boot-source": {
    "kernel_image_path": "/var/lib/firecracker/vmlinux-5.10",
    "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
  },
  "drives": [
    {
      "drive_id": "rootfs",
      "path_on_host": "/var/lib/firecracker/images/universal-runtime.ext4",
      "is_root_device": true,
      "is_read_only": true
    }
  ],
  "machine-config": {
    "vcpu_count": 2,
    "mem_size_mib": 2048,
    "track_dirty_pages": false
  },
  "network-interfaces": [],
  "vsock": {
    "guest_cid": 3,
    "uds_path": "/tmp/firecracker-vsock.sock"
  }
}
```

### 4. Universal Supervisor Design

A single Rust supervisor runs inside each VM, providing secure orchestration for all languages.

#### Sandbox Supervisor Architecture

```rust
// Single supervisor that handles all execution types
pub struct SandboxSupervisor {
    execution_engine: ExecutionEngine,
    workspace_manager: WorkspaceManager,
    telemetry_collector: TelemetryCollector,
    security_layer: SecurityLayer,
    language_plugins: HashMap<Language, Box<dyn LanguagePlugin>>,
}

impl SandboxSupervisor {
    pub async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult> {
        // 1. Set up security constraints (same for all languages)
        let sandbox = self.security_layer.prepare_sandbox(&request.constraints)?;

        // 2. Prepare workspace based on source type
        let workspace = self.workspace_manager
            .setup_workspace(&request)
            .await?;

        // 3. Get language plugin for specific handling (if needed)
        let plugin = self.language_plugins.get(&request.language);

        // 4. Execute with comprehensive telemetry
        let result = self.telemetry_collector.trace_execution(request.id, async {
            self.execution_engine.run(
                &workspace,
                &request.entry_point,
                &request.arguments,
                plugin.map(|p| p.as_ref()),
            ).await
        }).await?;

        // 5. Collect output artifacts
        let artifacts = self.collect_artifacts(&workspace, &request.outputs).await?;

        Ok(ExecutionResult {
            execution_id: request.id,
            exit_code: result.exit_code,
            stdout: result.stdout,
            stderr: result.stderr,
            artifacts,
            metrics: result.metrics,
            test_results: self.evaluate_tests(&result, &request.test_cases),
        })
    }
}
```

#### Workspace Management for Complex Projects

```rust
pub struct WorkspaceManager {
    scratch_root: PathBuf,
    max_workspace_size: u64,
}

impl WorkspaceManager {
    pub async fn setup_workspace(&self, request: &ExecutionRequest) -> Result<Workspace> {
        let workspace_path = self.create_isolated_directory()?;

        match &request.source {
            // Simple single-file execution
            ExecutionSource::Code { filename, content } => {
                fs::write(workspace_path.join(filename), content).await?;
            }

            // Multi-file project structure
            ExecutionSource::Files(files) => {
                for file in files {
                    let path = workspace_path.join(&file.path);
                    fs::create_dir_all(path.parent().unwrap()).await?;
                    fs::write(&path, &file.content).await?;
                    fs::set_permissions(&path, Permissions::from_mode(file.mode)).await?;
                }
            }

            // Clone from git repository
            ExecutionSource::GitRepo { url, branch, commit } => {
                self.clone_repository(url, branch, commit, &workspace_path).await?;
            }

            // Extract compressed archive
            ExecutionSource::Archive { data, format } => {
                self.extract_archive(data, format, &workspace_path).await?;
            }
        }

        Ok(Workspace {
            root: workspace_path,
            language: request.language,
            created_at: Instant::now(),
        })
    }
}
```

#### Execution Engine - Non-Intrusive Process Management

```rust
pub struct ExecutionEngine {
    process_monitor: ProcessMonitor,
    resource_limiter: ResourceLimiter,
}

impl ExecutionEngine {
    pub async fn run(
        &self,
        workspace: &Workspace,
        entry_point: &str,
        args: &[String],
        plugin: Option<&dyn LanguagePlugin>,
    ) -> Result<ExecutionOutput> {
        // Let plugin prepare environment if needed
        if let Some(plugin) = plugin {
            plugin.pre_execute(workspace).await?;
        }

        // Build command - plugin can customize or use defaults
        let mut cmd = plugin
            .map(|p| p.build_command(entry_point))
            .unwrap_or_else(|| self.default_command(workspace.language, entry_point));

        cmd.args(args)
           .current_dir(&workspace.root)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .kill_on_drop(true);

        // Apply resource constraints at process level
        self.resource_limiter.apply_to_command(&mut cmd)?;

        // Spawn and monitor process
        let mut child = cmd.spawn()?;
        let output = self.process_monitor.watch(&mut child).await?;

        // Let plugin process output if it needs to
        let language_metrics = if let Some(plugin) = plugin {
            plugin.process_output(&output).await
        } else {
            None
        };

        Ok(ExecutionOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            metrics: self.process_monitor.get_metrics(),
            language_metrics,
        })
    }

    fn default_command(&self, language: Language, entry_point: &str) -> Command {
        match language {
            Language::Python => Command::new("python3").arg(entry_point),
            Language::JavaScript => Command::new("node").arg(entry_point),
            Language::Go => Command::new("go").args(&["run", entry_point]),
            Language::Rust => Command::new("cargo").args(&["run"]),
            Language::Java => Command::new("java").arg(entry_point),
            Language::Cpp => Command::new(entry_point), // Assumes compiled
        }
    }
}
```

#### Extensible Language Plugin System

```rust
// Plugin trait for language-specific behavior
#[async_trait]
pub trait LanguagePlugin: Send + Sync {
    // Pre-execution setup (install dependencies, compile, etc)
    async fn pre_execute(&self, workspace: &Workspace) -> Result<()> {
        Ok(()) // Default: no special setup
    }

    // Build the execution command
    fn build_command(&self, entry_point: &str) -> Command;

    // Process output for language-specific metrics
    async fn process_output(&self, output: &Output) -> Option<LanguageMetrics> {
        None // Default: no special metrics
    }
}

// Example: Python plugin for complex projects
pub struct PythonPlugin;

#[async_trait]
impl LanguagePlugin for PythonPlugin {
    async fn pre_execute(&self, workspace: &Workspace) -> Result<()> {
        // Install requirements if present
        if workspace.has_file("requirements.txt") {
            Command::new("pip")
                .args(&["install", "-r", "requirements.txt", "--user"])
                .current_dir(&workspace.root)
                .output()
                .await?;
        }

        // Set up virtual environment if needed
        if workspace.has_file("Pipfile") {
            Command::new("pipenv")
                .args(&["install"])
                .current_dir(&workspace.root)
                .output()
                .await?;
        }

        Ok(())
    }

    fn build_command(&self, entry_point: &str) -> Command {
        // Check if using virtual environment
        if Path::new(".venv/bin/python").exists() {
            Command::new(".venv/bin/python").arg(entry_point)
        } else {
            Command::new("python3").arg(entry_point)
        }
    }
}

// Example: Node.js plugin
pub struct NodePlugin;

#[async_trait]
impl LanguagePlugin for NodePlugin {
    async fn pre_execute(&self, workspace: &Workspace) -> Result<()> {
        // Install npm dependencies
        if workspace.has_file("package.json") {
            Command::new("npm")
                .args(&["ci"]) // Use ci for faster, deterministic installs
                .current_dir(&workspace.root)
                .output()
                .await?;
        }
        Ok(())
    }

    fn build_command(&self, entry_point: &str) -> Command {
        // Handle npm scripts vs direct execution
        if entry_point.starts_with("npm ") {
            let parts: Vec<&str> = entry_point.split_whitespace().collect();
            Command::new("npm").args(&parts[1..])
        } else {
            Command::new("node").arg(entry_point)
        }
    }
}
```

### 5. Security Architecture

#### Defense in Depth with Supervisor

```
Layer 1: Hardware Isolation (Firecracker/KVM)
├── Separate kernel per execution
├── Hardware virtualization
└── No shared memory

Layer 2: Supervisor Enforcement (Rust)
├── Resource limit application
├── Process lifecycle management
├── Workspace isolation
└── Telemetry collection

Layer 3: Resource Constraints
├── CPU limits (cgroups)
├── Memory limits (cgroups)
├── Disk quotas
└── Process limits

Layer 4: Network Isolation
├── No network by default
├── Isolated network namespaces
└── Egress filtering if enabled

Layer 5: Filesystem Isolation
├── Read-only root filesystem
├── Temporary /tmp per execution
├── No persistent storage
└── Minimal attack surface

Layer 6: Syscall Filtering (seccomp-bpf)
├── Whitelist allowed syscalls
├── Block dangerous operations
└── Audit suspicious activity
```

#### Security Layer Implementation

````rust
pub struct SecurityLayer {
    cgroup_manager: CgroupManager,
    seccomp_filter: SeccompFilter,
    namespace_isolator: NamespaceIsolator,
}

impl SecurityLayer {
    pub fn prepare_sandbox(&self, constraints: &ExecutionConstraints) -> Result<Sandbox> {
        let sandbox = Sandbox::new();

        // Apply cgroup limits
        sandbox.set_memory_limit(constraints.memory_mb * 1024 * 1024);
        sandbox.set_cpu_quota(constraints.cpu_millicores);
        sandbox.set_pids_limit(100); // Prevent fork bombs

        // Filesystem isolation
        sandbox.mount_tmpfs("/tmp", "1G");
        sandbox.mount_tmpfs("/home/sandbox", "100M");
        sandbox.make_readonly(&["/usr", "/lib", "/bin", "/sbin"]);

        // Network isolation
        if !constraints.network_enabled {
            sandbox.disable_network();
        } else if let Some(rules) = &constraints.network_rules {
            sandbox.apply_network_rules(rules);
        }

        // Apply seccomp filter
        self.seccomp_filter.apply(SeccompProfile::Strict);

        Ok(sandbox)
    }
}

pub struct ResourceLimiter;

impl ResourceLimiter {
    pub fn apply_to_command(&self, cmd: &mut Command) -> Result<()> {
        // Set resource limits on the command
        unsafe {
            cmd.pre_exec(|| {
                // CPU time limit
                let rlimit = libc::rlimit {
                    rlim_cur: 300, // 5 minutes
                    rlim_max: 300,
                };
                libc::setrlimit(libc::RLIMIT_CPU, &rlimit);

                // Memory limit (set by cgroups, but also here as backup)
                let rlimit = libc::rlimit {
                    rlim_cur: 8 * 1024 * 1024 * 1024, // 8GB
                    rlim_max: 8 * 1024 * 1024 * 1024,
                };
                libc::setrlimit(libc::RLIMIT_AS, &rlimit);

                // No new processes
                let rlimit = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                libc::setrlimit(libc::RLIMIT_NPROC, &rlimit);

                Ok(())
            });
        }

        Ok(())
    }
}

#### Seccomp Policy

```json
{
  "default_action": "SCMP_ACT_ERRNO",
  "architectures": ["SCMP_ARCH_X86_64"],
  "syscalls": [
    {
      "names": ["read", "write", "close", "fstat", "lseek"],
      "action": "SCMP_ACT_ALLOW"
    },
    {
      "names": ["mmap", "mprotect", "munmap", "brk"],
      "action": "SCMP_ACT_ALLOW"
    },
    {
      "names": ["rt_sigaction", "rt_sigprocmask", "sigaltstack"],
      "action": "SCMP_ACT_ALLOW"
    },
    {
      "names": ["getcwd", "getpid", "getppid", "getuid", "geteuid"],
      "action": "SCMP_ACT_ALLOW"
    },
    { "names": ["dup", "dup2", "fcntl", "ioctl"], "action": "SCMP_ACT_ALLOW" },
    {
      "names": ["select", "poll", "epoll_create", "epoll_wait"],
      "action": "SCMP_ACT_ALLOW"
    },
    { "names": ["exit", "exit_group"], "action": "SCMP_ACT_ALLOW" },
    { "names": ["clock_gettime", "gettimeofday"], "action": "SCMP_ACT_ALLOW" }
  ]
}
````

### 6. Telemetry & Training Data Collection

#### Non-Intrusive Metrics Collection

```rust
pub struct TelemetryCollector {
    system_monitor: SystemMonitor,
    process_tracker: ProcessTracker,
}

impl TelemetryCollector {
    pub async fn trace_execution<F, T>(&self, execution_id: Uuid, f: F) -> Result<(T, ExecutionMetrics)>
    where
        F: Future<Output = Result<T>>
    {
        let start = Instant::now();

        // Capture initial system state
        let initial_state = SystemState {
            cpu_usage: self.system_monitor.get_cpu_usage(),
            memory_usage: self.system_monitor.get_memory_usage(),
            disk_io: self.system_monitor.get_disk_io_stats(),
            network_io: self.system_monitor.get_network_io_stats(),
        };

        // Execute with monitoring
        let result = f.await?;

        // Calculate deltas
        let final_state = self.system_monitor.get_current_state();

        let metrics = ExecutionMetrics {
            execution_id,
            duration: start.elapsed(),
            timestamps: self.get_phase_timestamps(),
            resource_usage: ResourceMetrics {
                cpu_time_ms: (final_state.cpu_usage - initial_state.cpu_usage).as_millis() as u64,
                memory_peak_mb: self.process_tracker.get_peak_memory_mb(),
                disk_read_bytes: final_state.disk_io.read_bytes - initial_state.disk_io.read_bytes,
                disk_write_bytes: final_state.disk_io.write_bytes - initial_state.disk_io.write_bytes,
                network_rx_bytes: final_state.network_io.rx_bytes - initial_state.network_io.rx_bytes,
                network_tx_bytes: final_state.network_io.tx_bytes - initial_state.network_io.tx_bytes,
            },
            process_metrics: ProcessMetrics {
                syscalls_count: self.get_seccomp_syscall_count(),
                context_switches: self.process_tracker.get_context_switches(),
                page_faults: self.process_tracker.get_page_faults(),
                cpu_migrations: self.process_tracker.get_cpu_migrations(),
            },
        };

        Ok((result, metrics))
    }
}

pub struct ProcessMonitor {
    pid: Option<u32>,
    start_time: Instant,
}

impl ProcessMonitor {
    pub async fn watch(&mut self, child: &mut Child) -> Result<Output> {
        self.pid = Some(child.id());
        self.start_time = Instant::now();

        // Monitor process while it runs
        let output = child.wait_with_output().await?;

        Ok(Output {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
            duration: self.start_time.elapsed(),
            peak_memory_mb: self.get_peak_memory_mb(),
        })
    }

    fn get_peak_memory_mb(&self) -> u64 {
        // Read from /proc/{pid}/status or cgroup memory.peak
        if let Some(pid) = self.pid {
            if let Ok(status) = fs::read_to_string(format!("/proc/{}/status", pid)) {
                // Parse VmPeak from status file
                for line in status.lines() {
                    if line.starts_with("VmPeak:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return kb / 1024; // Convert to MB
                            }
                        }
                    }
                }
            }
        }
        0
    }
}
```

#### Training Data Pipeline

```rust
pub struct TrainingDataCollector {
    telemetry_buffer: Arc<RwLock<Vec<ExecutionTelemetry>>>,
    s3_client: S3Client,
    batch_size: usize,
}

impl TrainingDataCollector {
    pub async fn process_execution(&self, telemetry: ExecutionTelemetry) {
        // 1. Enrich with additional metadata
        let enriched = self.enrich_telemetry(telemetry).await;

        // 2. Add to buffer
        let should_flush = {
            let mut buffer = self.telemetry_buffer.write().await;
            buffer.push(enriched);
            buffer.len() >= self.batch_size
        };

        // 3. Flush if needed
        if should_flush {
            self.flush_to_s3().await.ok();
        }
    }

    async fn flush_to_s3(&self) -> Result<()> {
        let batch = {
            let mut buffer = self.telemetry_buffer.write().await;
            std::mem::take(&mut *buffer)
        };

        if batch.is_empty() {
            return Ok(());
        }

        // Convert to Parquet for efficient storage
        let parquet_data = self.convert_to_parquet(&batch)?;

        // Upload to S3 with timestamp partitioning
        let key = format!(
            "training-data/year={}/month={}/day={}/batch-{}.parquet",
            chrono::Utc::now().year(),
            chrono::Utc::now().month(),
            chrono::Utc::now().day(),
            Uuid::new_v4()
        );

        self.s3_client
            .put_object()
            .bucket("shipd-training-data")
            .key(key)
            .body(parquet_data.into())
            .send()
            .await?;

        Ok(())
    }
}
```

## Production Infrastructure

### Multi-Region Deployment

```yaml
regions:
  primary:
    name: us-east-1
    availability_zones: [us-east-1a, us-east-1b, us-east-1c]
    capacity:
      compute_nodes: 50
      total_vcpus: 3200
      total_memory_gb: 12800

  secondary:
    - name: us-west-2
      availability_zones: [us-west-2a, us-west-2b]
      capacity:
        compute_nodes: 30
        total_vcpus: 1920
        total_memory_gb: 7680

    - name: eu-west-1
      availability_zones: [eu-west-1a, eu-west-1b]
      capacity:
        compute_nodes: 20
        total_vcpus: 1280
        total_memory_gb: 5120

global_load_balancer:
  provider: cloudflare
  health_check_interval: 10s
  failover_threshold: 3
  routing_policy: latency_based
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: execution-orchestrator
  namespace: shipd-execution
spec:
  replicas: 10
  selector:
    matchLabels:
      app: execution-orchestrator
  template:
    metadata:
      labels:
        app: execution-orchestrator
    spec:
      nodeSelector:
        node-type: control-plane

      containers:
        - name: orchestrator
          image: shipd/execution-orchestrator:v1.0.0
          resources:
            requests:
              cpu: 2
              memory: 4Gi
            limits:
              cpu: 4
              memory: 8Gi

          env:
            - name: RUST_LOG
              value: info
            - name: VM_POOL_SIZE
              value: "1000"
            - name: TELEMETRY_BATCH_SIZE
              value: "100"

          ports:
            - containerPort: 8080
              name: grpc
            - containerPort: 9090
              name: metrics

          livenessProbe:
            grpcHealthCheck:
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10

          readinessProbe:
            grpcHealthCheck:
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 5

---
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: sandbox-runtime
  namespace: shipd-execution
spec:
  selector:
    matchLabels:
      app: sandbox-runtime
  template:
    metadata:
      labels:
        app: sandbox-runtime
    spec:
      nodeSelector:
        node-type: sandbox-executor

      hostPID: true
      hostNetwork: true

      containers:
        - name: runtime
          image: shipd/sandbox-runtime:v1.0.0
          securityContext:
            privileged: true

          resources:
            requests:
              cpu: 4
              memory: 8Gi
            limits:
              cpu: 8
              memory: 16Gi

          volumeMounts:
            - name: dev
              mountPath: /dev
            - name: containerd-socket
              mountPath: /run/containerd/containerd.sock
            - name: firecracker-images
              mountPath: /var/lib/firecracker

          env:
            - name: FIRECRACKER_BIN
              value: /usr/local/bin/firecracker
            - name: JAILER_BIN
              value: /usr/local/bin/jailer
            - name: VM_POOL_CONFIG
              value: /etc/shipd/vm-pool.yaml

      volumes:
        - name: dev
          hostPath:
            path: /dev
        - name: containerd-socket
          hostPath:
            path: /run/containerd/containerd.sock
        - name: firecracker-images
          hostPath:
            path: /var/lib/firecracker
```

### Database Schema

```sql
-- Execution tracking database
CREATE TABLE executions (
    id UUID PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    problem_id VARCHAR(255) NOT NULL,
    language VARCHAR(50) NOT NULL,
    source_type VARCHAR(50) NOT NULL, -- 'code', 'files', 'git', 'archive'
    entry_point VARCHAR(500) NOT NULL,
    status VARCHAR(50) NOT NULL,

    -- Timing information
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    -- Results
    exit_code INTEGER,
    all_tests_passed BOOLEAN,
    tests_run INTEGER,
    tests_passed INTEGER,

    -- Resource usage
    cpu_time_ms INTEGER,
    memory_peak_mb INTEGER,
    disk_read_bytes BIGINT,
    disk_write_bytes BIGINT,

    -- Indexes for performance
    INDEX idx_user_executions (user_id, created_at DESC),
    INDEX idx_problem_executions (problem_id, created_at DESC),
    INDEX idx_status (status, created_at DESC),
    INDEX idx_language (language, created_at DESC)
);

CREATE TABLE execution_sources (
    id UUID PRIMARY KEY,
    execution_id UUID NOT NULL REFERENCES executions(id),
    source_type VARCHAR(50) NOT NULL,

    -- For code type
    filename VARCHAR(255),
    content TEXT,

    -- For git type
    git_url VARCHAR(500),
    git_branch VARCHAR(100),
    git_commit VARCHAR(40),

    -- For archive type
    archive_format VARCHAR(20),
    archive_size_bytes INTEGER,

    -- For files type (see execution_files table)

    INDEX idx_execution_source (execution_id)
);

CREATE TABLE execution_files (
    id UUID PRIMARY KEY,
    execution_id UUID NOT NULL REFERENCES executions(id),
    file_path VARCHAR(500) NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    file_hash VARCHAR(64),

    INDEX idx_execution_files (execution_id),
    INDEX idx_file_path (execution_id, file_path)
);

CREATE TABLE execution_results (
    id UUID PRIMARY KEY,
    execution_id UUID NOT NULL REFERENCES executions(id),
    test_case_name VARCHAR(255) NOT NULL,
    passed BOOLEAN NOT NULL,
    runtime_ms INTEGER NOT NULL,
    memory_used_mb INTEGER,

    -- Test output
    stdout TEXT,
    stderr TEXT,
    exit_code INTEGER,
    error_type VARCHAR(100),
    error_message TEXT,

    -- Artifacts
    output_files JSONB, -- {filename: content} for small files

    INDEX idx_execution_results (execution_id)
);

CREATE TABLE execution_telemetry (
    id UUID PRIMARY KEY,
    execution_id UUID NOT NULL REFERENCES executions(id),

    -- Phase timings
    vm_acquisition_ms INTEGER,
    workspace_setup_ms INTEGER,
    execution_ms INTEGER,
    cleanup_ms INTEGER,

    -- Resource metrics
    cpu_time_ms BIGINT,
    memory_peak_mb INTEGER,
    disk_read_bytes BIGINT,
    disk_write_bytes BIGINT,
    network_rx_bytes BIGINT,
    network_tx_bytes BIGINT,

    -- Process metrics
    syscalls_count INTEGER,
    context_switches INTEGER,
    page_faults INTEGER,
    cpu_migrations INTEGER,

    -- Language-specific metrics (from plugins)
    language_metrics JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_telemetry_execution (execution_id)
);
```

## Performance Optimization

### VM Pool Pre-warming

```rust
pub struct VmPrewarmer {
    pool: Arc<VmPool>,
    predictor: Arc<LoadPredictor>,
}

impl VmPrewarmer {
    pub async fn run(&self) {
        loop {
            // Predict load for next 5 minutes
            let predictions = self.predictor.predict_load(Duration::from_secs(300)).await;

            for (language, predicted_load) in predictions {
                let current_warm = self.pool.get_warm_count(language).await;
                let target = (predicted_load * 1.2) as usize; // 20% buffer

                if target > current_warm {
                    let to_create = target - current_warm;
                    self.spawn_vms(language, to_create).await;
                }
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
```

### Memory Deduplication

```rust
// Enable KSM (Kernel Same-page Merging) for VMs
pub fn enable_memory_deduplication() -> Result<()> {
    // Enable KSM
    fs::write("/sys/kernel/mm/ksm/run", "1")?;

    // Aggressive scanning for sandbox workloads
    fs::write("/sys/kernel/mm/ksm/sleep_millisecs", "20")?;
    fs::write("/sys/kernel/mm/ksm/pages_to_scan", "1000")?;

    Ok(())
}
```

### Execution Caching

```rust
pub struct ExecutionCache {
    redis: Arc<RedisClient>,
    ttl: Duration,
}

impl ExecutionCache {
    pub async fn get_cached_result(
        &self,
        code_hash: &str,
        test_hash: &str
    ) -> Option<ExecutionResult> {
        let key = format!("exec:{}:{}", code_hash, test_hash);

        self.redis
            .get::<_, Option<Vec<u8>>>(key)
            .await
            .ok()
            .flatten()
            .and_then(|data| bincode::deserialize(&data).ok())
    }

    pub async fn cache_result(
        &self,
        code_hash: &str,
        test_hash: &str,
        result: &ExecutionResult
    ) -> Result<()> {
        let key = format!("exec:{}:{}", code_hash, test_hash);
        let data = bincode::serialize(result)?;

        self.redis
            .set_ex(key, data, self.ttl.as_secs() as usize)
            .await?;

        Ok(())
    }
}
```

## Monitoring & Observability

### Key Metrics

```yaml
metrics:
  # Execution metrics
  - name: execution_requests_total
    type: counter
    labels: [language, status]

  - name: execution_duration_seconds
    type: histogram
    buckets: [0.1, 0.5, 1, 2, 5, 10, 30]
    labels: [language]

  - name: vm_pool_size
    type: gauge
    labels: [language, state]

  - name: vm_acquisition_duration_seconds
    type: histogram
    buckets: [0.01, 0.05, 0.1, 0.5, 1]

  # Resource metrics
  - name: cpu_usage_percent
    type: gauge
    labels: [node]

  - name: memory_usage_bytes
    type: gauge
    labels: [node]

  - name: active_vms_count
    type: gauge
    labels: [node, language]

  # Training data metrics
  - name: telemetry_events_collected_total
    type: counter

  - name: telemetry_batch_size_bytes
    type: histogram
    buckets: [1000, 10000, 100000, 1000000]
```

### Grafana Dashboards

```json
{
  "dashboard": {
    "title": "Shipd Code Execution Platform",
    "panels": [
      {
        "title": "Execution Rate",
        "targets": [
          {
            "expr": "rate(execution_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Execution Latency (p99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, execution_duration_seconds)"
          }
        ]
      },
      {
        "title": "VM Pool Utilization",
        "targets": [
          {
            "expr": "vm_pool_size{state='warm'} / vm_pool_size{state='total'}"
          }
        ]
      },
      {
        "title": "Success Rate by Language",
        "targets": [
          {
            "expr": "rate(execution_requests_total{status='success'}[5m]) / rate(execution_requests_total[5m])"
          }
        ]
      }
    ]
  }
}
```

## Security Considerations

### Threat Model

1. **Malicious Code Execution**

   - **Threat**: User submits code designed to escape sandbox
   - **Mitigation**: Hardware isolation via Firecracker, seccomp filters, resource limits

2. **Resource Exhaustion**

   - **Threat**: Fork bombs, memory exhaustion, CPU spinning
   - **Mitigation**: Strict resource limits, process count limits, timeout enforcement

3. **Data Exfiltration**

   - **Threat**: Attempting to access other users' data
   - **Mitigation**: Complete isolation between executions, no persistent storage

4. **Network Attacks**
   - **Threat**: Using execution environment for network attacks
   - **Mitigation**: Network disabled by default, egress filtering when enabled

### Compliance & Auditing

```rust
pub struct AuditLogger {
    writer: Arc<AuditWriter>,
}

impl AuditLogger {
    pub async fn log_execution(&self, event: ExecutionAuditEvent) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type: "code_execution",
            user_id: event.user_id,
            ip_address: event.ip_address,
            execution_id: event.execution_id,
            language: event.language,
            code_hash: event.code_hash,
            result: event.result,
            resource_usage: event.resource_usage,
        };

        self.writer.write(entry).await;
    }
}
```

## Development & Testing

### Local Development Environment

```yaml
# docker-compose.yml for local development
version: "3.8"

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: shipd_execution
      POSTGRES_USER: shipd
      POSTGRES_PASSWORD: development
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  orchestrator:
    build:
      context: .
      dockerfile: Dockerfile.orchestrator
    environment:
      DATABASE_URL: postgres://shipd:development@postgres/shipd_execution
      REDIS_URL: redis://redis:6379
      SANDBOX_MODE: mock # Use mock sandboxes for local dev
    ports:
      - "8080:8080"
      - "9090:9090"
    depends_on:
      - postgres
      - redis

  # Mock sandbox for local testing
  mock-sandbox:
    build:
      context: .
      dockerfile: Dockerfile.mock-sandbox
    environment:
      ORCHESTRATOR_URL: http://orchestrator:8080
    depends_on:
      - orchestrator
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_execution() {
        let executor = create_test_executor();

        let request = ExecutionRequest {
            language: Language::Python,
            code: r#"
def solution(n):
    return n * 2

print(solution(21))
"#.to_string(),
            test_cases: vec![TestCase {
                input: "".to_string(),
                expected_output: "42\n".to_string(),
                timeout_ms: Some(1000),
                is_hidden: false,
            }],
            ..Default::default()
        };

        let result = executor.execute(request).await.unwrap();

        assert!(result.all_tests_passed);
        assert_eq!(result.test_results[0].stdout, "42\n");
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let executor = create_test_executor();

        let request = ExecutionRequest {
            language: Language::Python,
            code: r#"
# Attempt to use excessive memory
data = [0] * (10**9)  # Try to allocate 1GB
"#.to_string(),
            constraints: ExecutionConstraints {
                memory_mb: 128, // Only allow 128MB
                ..Default::default()
            },
            ..Default::default()
        };

        let result = executor.execute(request).await.unwrap();

        assert!(!result.success);
        assert!(result.error_message.contains("MemoryError"));
    }
}
```

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)

- [ ] Core execution orchestrator in Rust
- [ ] Basic Firecracker integration
- [ ] Python and JavaScript runtime support
- [ ] Simple test case evaluation

### Phase 2: Security & Isolation (Weeks 3-4)

- [ ] Complete seccomp policies
- [ ] Resource limit enforcement
- [ ] Network isolation
- [ ] Security testing suite

### Phase 3: Performance (Weeks 5-6)

- [ ] VM pool pre-warming
- [ ] Execution caching
- [ ] Memory deduplication
- [ ] Load prediction

### Phase 4: Scale & Production (Weeks 7-8)

- [ ] Multi-region deployment
- [ ] Kubernetes operators
- [ ] Monitoring dashboards
- [ ] Load testing at scale

### Phase 5: Training Data (Weeks 9-10)

- [ ] Telemetry collection pipeline
- [ ] S3 data lake integration
- [ ] Parquet conversion
- [ ] Data quality validation

### Phase 6: Integration (Weeks 11-12)

- [ ] Shipd platform integration
- [ ] API gateway setup
- [ ] End-to-end testing
- [ ] Production deployment

## Cost Analysis

### Infrastructure Costs (Monthly)

```yaml
compute:
  # 50 c5.metal instances for primary region
  primary_region:
    instance_type: c5.metal
    count: 50
    cost_per_instance: $4,080
    total: $204,000

  # 30 c5.metal instances for secondary regions
  secondary_regions:
    instance_type: c5.metal
    count: 30
    cost_per_instance: $4,080
    total: $122,400

  # Control plane (Kubernetes)
  control_plane:
    instance_type: m5.2xlarge
    count: 15
    cost_per_instance: $276
    total: $4,140

storage:
  # Execution data and telemetry
  s3:
    size_tb: 100
    cost_per_tb: $23
    total: $2,300

  # Databases
  rds:
    instance_type: db.r5.4xlarge
    multi_az: true
    total: $2,300

networking:
  # Data transfer
  egress_gb: 10000
  cost_per_gb: $0.09
  total: $900

monitoring:
  # Datadog/CloudWatch
  total: $5,000

total_monthly: $341,040
cost_per_million_executions: $34.10
```

### Optimization Opportunities

1. **Spot Instances**: Use spot for non-critical workloads (30% savings)
2. **Reserved Instances**: 3-year commits for baseline capacity (60% savings)
3. **Graviton Instances**: ARM-based instances where applicable (20% savings)
4. **Intelligent Tiering**: Move cold data to cheaper storage automatically

## Conclusion

This architecture provides DataCurve with a world-class code execution platform that:

1. **Scales infinitely** through horizontal scaling and multi-region deployment
2. **Executes securely** with hardware-level isolation and defense in depth
3. **Performs exceptionally** with sub-200ms cold starts and high throughput
4. **Collects rich data** for LLM training without impacting performance
5. **Operates reliably** with 99.99% availability through redundancy
6. **Integrates seamlessly** with the existing Shipd platform

The modular design allows for incremental implementation while maintaining the ability to scale to millions of executions per day. The comprehensive telemetry system ensures that every execution provides valuable training data for improving code generation models.

With this foundation, Shipd can confidently onboard developers worldwide, knowing that their code will execute quickly, securely, and reliably while generating invaluable data for advancing the state of AI-assisted programming.
