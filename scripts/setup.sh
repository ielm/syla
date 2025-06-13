#!/bin/bash
# Syla Platform Setup Script
# Usage: curl -sSf https://your-domain.com/setup.sh | sh
# Or: curl -sSf https://your-domain.com/setup.sh | sh -s -- --path /custom/path

set -e

# Configuration
SYLA_REPO="https://github.com/ielm/syla.git"
RUST_MIN_VERSION="1.70.0"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Print functions
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

# Banner
print_banner() {
    echo ""
    echo "   ███████╗██╗   ██╗██╗      █████╗ "
    echo "   ██╔════╝╚██╗ ██╔╝██║     ██╔══██╗"
    echo "   ███████╗ ╚████╔╝ ██║     ███████║"
    echo "   ╚════██║  ╚██╔╝  ██║     ██╔══██║"
    echo "   ███████║   ██║   ███████╗██║  ██║"
    echo "   ╚══════╝   ╚═╝   ╚══════╝╚═╝  ╚═╝"
    echo ""
    echo "   High-Performance Code Execution Platform"
    echo ""
}

# Get user information
get_user_info() {
    UNIX_NAME=$(whoami)
    HOSTNAME=$(hostname)
    info "Setting up Syla for user: ${UNIX_NAME}@${HOSTNAME}"
}

# Parse command line arguments
parse_args() {
    WORKSPACE_PATH=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --path)
                WORKSPACE_PATH="$2"
                shift 2
                ;;
            --help|-h)
                echo "Syla Platform Setup"
                echo ""
                echo "Usage:"
                echo "  curl -sSf https://your-domain.com/setup.sh | sh"
                echo "  curl -sSf https://your-domain.com/setup.sh | sh -s -- --path /custom/path"
                echo ""
                echo "Options:"
                echo "  --path <PATH>    Specify installation directory (default: ./syla-workspace)"
                echo "  --help, -h       Show this help message"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Determine workspace directory
determine_workspace() {
    if [[ -z "$WORKSPACE_PATH" ]]; then
        echo ""
        echo "Where would you like to create the Syla workspace?"
        echo "  - Press ENTER for default: ./syla-workspace"
        echo "  - Type '.' for current directory"
        echo "  - Or enter a custom path"
        echo ""
        read -p "Installation directory: " USER_PATH
        
        if [[ -z "$USER_PATH" ]]; then
            WORKSPACE_PATH="syla-workspace"
        elif [[ "$USER_PATH" == "." ]]; then
            WORKSPACE_PATH="."
        else
            WORKSPACE_PATH="$USER_PATH"
        fi
    fi
    
    # Convert to absolute path
    if [[ "$WORKSPACE_PATH" == "." ]]; then
        WORKSPACE_PATH=$(pwd)
    elif [[ ! "$WORKSPACE_PATH" = /* ]]; then
        WORKSPACE_PATH="$(pwd)/$WORKSPACE_PATH"
    fi
    
    info "Workspace will be created at: $WORKSPACE_PATH"
}

# Check system requirements
check_requirements() {
    info "Checking system requirements..."
    
    # Check OS
    OS=$(uname -s)
    case "$OS" in
        Linux|Darwin)
            success "Operating system: $OS"
            ;;
        *)
            error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac
    
    # Check Git
    if ! command -v git &> /dev/null; then
        error "Git is not installed. Please install Git first."
        exit 1
    fi
    success "Git: $(git --version)"
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    success "Docker: $(docker --version)"
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        warning "Rust is not installed."
        echo ""
        read -p "Would you like to install Rust? (y/n) " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            info "Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
            success "Rust installed successfully"
        else
            error "Rust is required for Syla. Please install Rust and try again."
            exit 1
        fi
    else
        success "Rust: $(rustc --version)"
    fi
    
    # Check GitHub CLI (optional)
    if command -v gh &> /dev/null; then
        success "GitHub CLI: $(gh --version | head -n1)"
    else
        warning "GitHub CLI not found (optional)"
    fi
    
    echo ""
}

# Create workspace
create_workspace() {
    info "Creating workspace..."
    
    # Create directory if it doesn't exist
    if [[ ! -d "$WORKSPACE_PATH" ]]; then
        mkdir -p "$WORKSPACE_PATH"
    fi
    
    cd "$WORKSPACE_PATH"
    
    # Clone the repository
    if [[ -d ".git" ]]; then
        info "Workspace already exists, pulling latest changes..."
        git pull origin main
    else
        info "Cloning Syla repository..."
        git clone "$SYLA_REPO" .
    fi
    
    success "Workspace created at: $WORKSPACE_PATH"
}

# Build the meta-CLI
build_cli() {
    info "Building Syla CLI..."
    
    cd "$WORKSPACE_PATH/cli"
    cargo build --release
    
    # Create symlink in workspace root
    cd "$WORKSPACE_PATH"
    ln -sf cli/target/release/syla syla
    
    success "Syla CLI built successfully"
}

# Set up shell integration
setup_shell() {
    info "Setting up shell integration..."
    
    # Detect shell
    SHELL_NAME=$(basename "$SHELL")
    case "$SHELL_NAME" in
        bash)
            RC_FILE="$HOME/.bashrc"
            ;;
        zsh)
            RC_FILE="$HOME/.zshrc"
            ;;
        fish)
            RC_FILE="$HOME/.config/fish/config.fish"
            ;;
        *)
            warning "Unknown shell: $SHELL_NAME. Please add $WORKSPACE_PATH to your PATH manually."
            return
            ;;
    esac
    
    # Add to PATH if not already present
    if ! grep -q "export PATH=\"$WORKSPACE_PATH:\$PATH\"" "$RC_FILE" 2>/dev/null; then
        echo "" >> "$RC_FILE"
        echo "# Syla Platform" >> "$RC_FILE"
        echo "export PATH=\"$WORKSPACE_PATH:\$PATH\"" >> "$RC_FILE"
        success "Added Syla to PATH in $RC_FILE"
        echo ""
        warning "Please run: source $RC_FILE"
        warning "Or restart your terminal to use the 'syla' command"
    else
        info "Syla already in PATH"
    fi
}

# Final instructions
print_instructions() {
    echo ""
    echo "════════════════════════════════════════════════════════════════"
    success "Syla Platform setup complete!"
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    echo "Next steps:"
    echo "  1. Source your shell config or restart your terminal"
    echo "  2. Run 'syla doctor' to verify installation"
    echo "  3. Run 'syla init' to clone platform repositories"
    echo "  4. Run 'syla dev up' to start development environment"
    echo ""
    echo "Workspace location: $WORKSPACE_PATH"
    echo "Documentation: https://github.com/ielm/syla"
    echo ""
    echo "Happy coding! 🚀"
    echo ""
}

# Main installation flow
main() {
    print_banner
    get_user_info
    parse_args "$@"
    determine_workspace
    check_requirements
    create_workspace
    build_cli
    setup_shell
    print_instructions
}

# Run main function
main "$@"