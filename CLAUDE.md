# CLAUDE.md - Syla Platform

This file provides guidance to Claude Code when working with the Syla platform workspace.

## Getting Started

Always start your session by checking system health and workspace status:

```bash
./syla doctor && ./syla status
cat .eva/blackboard.md
```

## Workspace Structure

This is a **meta-platform workspace** that orchestrates multiple repositories:

```
syla/                          # Parent workspace (you are here)
├── platforms/                 # Product platforms
│   └── syla/                 # Code execution platform
│       ├── core/            # Core services
│       │   ├── api-gateway/
│       │   └── execution-service/
│       └── tools/           # Platform tools
│           └── cli/
├── .platform/                 # Meta-platform tooling
│   ├── syla-cli/            # Workspace management CLI (Rust)
│   └── config/
│       └── repos.toml       # Repository manifest
└── .eva/                     # Persistent AI memory
```

## Key Commands

### Workspace Management
```bash
./syla init              # Clone all repositories
./syla init -p syla      # Clone only Syla platform repos
./syla status            # Show repository and service status
./syla doctor            # Check system health
```

### Development
```bash
./syla dev up            # Start all services (not yet implemented)
./syla dev logs <service># View service logs (not yet implemented)
```

## Repository Naming Convention

- **Local directories**: No prefix (e.g., `cli`, `api-gateway`)
- **GitHub repositories**: With prefix (e.g., `syla-cli`, `syla-api-gateway`)
- **Full reference**: Platform.component (e.g., `syla.core.api-gateway`)

## Working with Services

Current services and their ports:
- **API Gateway**: http://localhost:8084
- **Execution Service**: http://localhost:8083
- **Redis**: localhost:6380
- **PostgreSQL**: localhost:5434

## Eva System

The `.eva/` directory contains persistent memory across sessions:
- `blackboard.md` - Current state and tasks
- `docs/` - Architecture documentation
- `projects/` - Project-specific workspaces

Key Eva commands:
```bash
# Check current context
cat .eva/blackboard.md

# Architecture docs
ls .eva/docs/architecture/
```

## Development Workflow

1. Check system: `./syla doctor`
2. Check status: `./syla status`  
3. Make changes in service directories
4. Test locally
5. Update blackboard
6. Commit changes in individual repos

## Important Notes

- This is a polyrepo architecture - each service is its own git repository
- The parent workspace will show child repos as untracked in git (this is expected)
- Always prefer Rust for new tooling and platform code
- Services communicate via gRPC (future) and REST (current)

## Current Implementation Status

- ✅ Meta-CLI (`syla`) with basic commands
- ✅ Repository manifest system
- ✅ API Gateway with REST endpoints
- ✅ Execution Service with Docker sandboxing
- ✅ CLI with local and remote execution
- 🚧 Platform orchestration commands
- 🚧 Unified development environment
- 📋 Firecracker integration (planned)
- 📋 Telemetry system (planned)

## Architecture References

- Platform Architecture: `.eva/docs/architecture/platform-architecture.md`
- CLI Specification: `.eva/docs/architecture/cli-specification.md`
- Service Architecture: `.eva/docs/architecture/service-architecture.md`
- Workspace Strategy: `.eva/docs/architecture/workspace-strategy.md`
- Implementation Roadmap: `.eva/docs/architecture/implementation-roadmap.md`