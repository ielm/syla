# Syla Platform Quick Start

## MVP Status

We have a working CLI that can execute code in Docker containers. The service architecture is partially implemented.

## Testing the CLI

### 1. Build the CLI

```bash
cd syla-cli
cargo build
cd ..
```

### 2. Initialize Workspace (Optional - we already have one)

```bash
# This is now safe - it will detect existing workspace
./syla-cli/target/debug/syla init
```

### 3. Check System Health

```bash
./syla-cli/target/debug/syla doctor
```

### 4. Execute Code

```bash
# Python
./syla-cli/target/debug/syla exec test_hello.py

# Python with args
./syla-cli/target/debug/syla exec test_hello.py -- arg1 arg2

# JavaScript
./syla-cli/target/debug/syla exec test_javascript.js

# Go
./syla-cli/target/debug/syla exec test_hello.go

# With JSON output
./syla-cli/target/debug/syla exec -o json test_hello.py
```

## Architecture Overview

### Current Components

1. **syla-cli** - Command line interface (working)
   - Local Docker execution
   - Multiple language support
   - Configurable timeouts

2. **syla-api-gateway** - REST API gateway (partial)
   - Health endpoint
   - Execution endpoints (stubbed)

3. **syla-execution-service** - Execution orchestrator (partial)
   - Redis queue setup
   - Worker structure

### Data Flow (Target)

```
CLI -> API Gateway -> Execution Service -> Docker/Firecracker
                         |
                         v
                      Redis Queue
```

## Next Development Steps

1. Complete execution service worker
2. Wire up services communication
3. Test end-to-end flow
4. Add persistence layer

## Development Commands

```bash
# Start Redis and PostgreSQL
docker-compose up -d

# Run API Gateway (in separate terminal)
cd syla-api-gateway
cargo run

# Run Execution Service (in separate terminal)
cd syla-execution-service
cargo run
```