# MVP Implementation Plan

## Goal
Minimal end-to-end code execution system that can:
1. Accept code via CLI
2. Execute in a basic sandbox (Docker initially, Firecracker later)
3. Return results

## Components for MVP

### Phase 1: Core CLI & Local Execution (DONE ✓)
- `syla-cli` with basic commands ✓
- Local Docker-based execution ✓
- Simple workspace management ✓

### Phase 2: Service Architecture (Next)
- `syla-api-gateway` (minimal)
- `syla-execution-service` (queue + Docker)
- `syla-workspace-service` (ephemeral only)

### Phase 3: Production Features (Later)
- Firecracker integration
- Full workspace types
- Telemetry
- Auth

## Current Status
✅ CLI built and ready to test
✅ Docker execution implemented
✅ Basic workspace initialization

## Test Plan
1. Test `syla init` to create workspace
2. Test `syla doctor` to verify Docker
3. Test `syla exec test_hello.py`
4. Test with different languages (JS, Go)

## Architecture Decisions for MVP
- Start with REST API (simpler than gRPC)
- Use Redis for job queue
- PostgreSQL for execution history
- Docker for initial sandboxing
- Simple round-robin scheduling