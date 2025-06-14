#!/bin/bash
# Setup proto dependencies for the Syla platform
# Downloads googleapis and sets up proto structure

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

warning() {
    echo -e "${YELLOW}warning:${NC} $1"
}

# Get the workspace root
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROTO_DEPS_DIR="$WORKSPACE_ROOT/proto-deps"

info "Setting up proto dependencies in $WORKSPACE_ROOT"

# Create proto-deps directory
mkdir -p "$PROTO_DEPS_DIR"

# Download googleapis if not present or if forced
if [ ! -d "$PROTO_DEPS_DIR/googleapis" ] || [ "$1" == "--force" ]; then
    info "Downloading googleapis..."
    
    # Remove existing if force
    if [ "$1" == "--force" ] && [ -d "$PROTO_DEPS_DIR/googleapis" ]; then
        rm -rf "$PROTO_DEPS_DIR/googleapis"
    fi
    
    # Clone with sparse checkout for efficiency
    cd "$PROTO_DEPS_DIR"
    git clone --depth=1 --filter=blob:none --sparse https://github.com/googleapis/googleapis.git
    cd googleapis
    
    # Configure sparse checkout to only get what we need
    git sparse-checkout init --cone
    git sparse-checkout set google/api google/rpc google/type google/protobuf
    
    success "googleapis downloaded successfully"
else
    info "googleapis already present (use --force to re-download)"
fi

# Create proto-common directory if it doesn't exist
PROTO_COMMON_DIR="$WORKSPACE_ROOT/proto-common"
if [ ! -d "$PROTO_COMMON_DIR" ]; then
    info "Creating proto-common directory..."
    mkdir -p "$PROTO_COMMON_DIR/syla"
    success "proto-common directory created"
else
    info "proto-common directory already exists"
fi

# Create .gitignore for proto-deps if it doesn't exist
if [ ! -f "$PROTO_DEPS_DIR/.gitignore" ]; then
    echo "googleapis/" > "$PROTO_DEPS_DIR/.gitignore"
    success "Created .gitignore for proto-deps"
fi

success "Proto dependencies setup complete"

# Show next steps
echo ""
echo "Next steps:"
echo "1. Run './scripts/setup-service.sh <service-path>' to set up a specific service"
echo "2. Add shared proto definitions to proto-common/syla/"
echo "3. Use 'make setup' in each service directory to set up proto symlinks"