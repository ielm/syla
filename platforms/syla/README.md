# Syla Platform

The Syla platform provides secure code execution infrastructure for Shipd.

## Platform Structure

```
syla/
├── core/                 # Core platform services
│   ├── api-gateway/      # REST API gateway
│   ├── execution-service/# Code execution orchestrator
│   └── orchestrator/     # Service orchestration (future)
├── runtimes/            # Language-specific execution runtimes
│   ├── python-runtime/   # Python execution environment
│   ├── javascript-runtime/# JavaScript/Node.js environment
│   └── rust-runtime/     # Rust execution environment
├── sdks/                # Client SDKs
│   ├── typescript-sdk/   # TypeScript/JavaScript SDK
│   └── python-sdk/       # Python SDK
└── tools/               # Platform tools
    └── cli/             # Syla CLI
```

## Core Services

### API Gateway

- REST API for code execution requests
- Request routing and load balancing
- Authentication and rate limiting
- Telemetry collection

### Execution Service

- Manages code execution jobs
- Handles queuing and scheduling
- Integrates with Docker/Firecracker
- Collects execution metrics

## Development

All services are managed through the meta-platform CLI:

```bash
# Start all Syla services
syla platform start syla

# Check platform health
syla platform status syla

# View logs
syla platform logs syla/core/api-gateway
```

## Architecture Principles

1. **Microservices**: Each service has a single responsibility
2. **Language Agnostic**: Support multiple programming languages
3. **Secure by Default**: Sandboxed execution environments
4. **Observable**: Comprehensive telemetry and monitoring
5. **Scalable**: Horizontal scaling of all components

