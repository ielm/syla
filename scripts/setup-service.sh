#!/bin/bash
# Setup proto dependencies for a specific service
# Usage: ./scripts/setup-service.sh <service-path>

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}info:${NC} $1"
}

success() {
    echo -e "${GREEN}success:${NC} $1"
}

error() {
    echo -e "${RED}error:${NC} $1" >&2
}

# Check arguments
if [ $# -lt 1 ]; then
    error "Usage: $0 <service-path>"
    error "Example: $0 platforms/syla/core/api-gateway"
    exit 1
fi

SERVICE_PATH="$1"
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_DIR="$WORKSPACE_ROOT/$SERVICE_PATH"

# Check if service directory exists
if [ ! -d "$SERVICE_DIR" ]; then
    error "Service directory not found: $SERVICE_DIR"
    exit 1
fi

SERVICE_NAME=$(basename "$SERVICE_DIR")
info "Setting up proto dependencies for $SERVICE_NAME..."

cd "$SERVICE_DIR"

# Create proto directory if it doesn't exist
mkdir -p proto

# Create proto-deps directory if it doesn't exist
mkdir -p proto-deps

# Link googleapis from workspace proto-deps
if [ ! -e "proto-deps/googleapis" ]; then
    # Calculate relative path to workspace proto-deps
    RELATIVE_PATH=$(python3 -c "import os.path; print(os.path.relpath('$WORKSPACE_ROOT/proto-deps/googleapis', '$SERVICE_DIR/proto-deps'))")
    ln -sf "$RELATIVE_PATH" proto-deps/googleapis
    success "Linked googleapis from workspace"
else
    info "googleapis already linked"
fi

# Create symlinks in proto directory
cd proto

# Link google from proto-deps
if [ ! -e "google" ]; then
    ln -sf ../proto-deps/googleapis/google google
    success "Created google symlink in proto/"
else
    info "google symlink already exists"
fi

# Link common from workspace proto-common
if [ ! -e "common" ]; then
    # Calculate relative path to workspace proto-common
    RELATIVE_PATH=$(python3 -c "import os.path; print(os.path.relpath('$WORKSPACE_ROOT/proto-common', '$SERVICE_DIR/proto'))")
    ln -sf "$RELATIVE_PATH" common
    success "Created common symlink in proto/"
else
    info "common symlink already exists"
fi

# Create .gitignore for proto-deps if it doesn't exist
cd "$SERVICE_DIR"
if [ ! -f "proto-deps/.gitignore" ]; then
    echo "*" > proto-deps/.gitignore
    echo "!.gitignore" >> proto-deps/.gitignore
    success "Created .gitignore for proto-deps"
fi

# Generate Cargo.lock if it's a Rust service and doesn't exist
if [ -f "Cargo.toml" ] && [ ! -f "Cargo.lock" ]; then
    info "Generating Cargo.lock..."
    cargo generate-lockfile
    success "Generated Cargo.lock"
fi

success "Proto setup complete for $SERVICE_NAME"

# Show service structure
echo ""
echo "Service proto structure:"
echo "$SERVICE_DIR/"
echo "├── proto/"
echo "│   ├── google -> ../proto-deps/googleapis/google"
echo "│   ├── common -> $WORKSPACE_ROOT/proto-common"
echo "│   └── *.proto (service-specific protos)"
echo "└── proto-deps/"
echo "    └── googleapis -> $WORKSPACE_ROOT/proto-deps/googleapis"