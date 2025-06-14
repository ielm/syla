#!/bin/bash
# Detect which services have changed since last build
# Uses git to detect changes or build timestamps

set -e

# Get service directories based on pattern
find_services() {
    local base_dir="$1"
    local services=""
    
    # Find all directories containing Cargo.toml or package.json
    while IFS= read -r service_dir; do
        service_dir=$(dirname "$service_dir")
        # Get relative path from workspace root
        service_rel=$(realpath --relative-to="$base_dir" "$service_dir")
        services="$services $service_rel"
    done < <(find "$base_dir/platforms" -name "Cargo.toml" -o -name "package.json" 2>/dev/null | grep -E "(core|tools|services)" | sort -u)
    
    echo $services
}

# Function to check if a service has changed
has_changed() {
    local service=$1
    local workspace_root=$2
    
    # If we're not in a git repo, assume all changed
    if ! git -C "$workspace_root" rev-parse --git-dir > /dev/null 2>&1; then
        return 0
    fi
    
    # Check if service directory exists
    if [ ! -d "$workspace_root/$service" ]; then
        return 1
    fi
    
    cd "$workspace_root"
    
    # Get the last commit that touched this service
    last_commit=$(git log -1 --format="%H" -- "$service" 2>/dev/null || echo "")
    
    if [ -z "$last_commit" ]; then
        return 0  # No commits, consider it changed
    fi
    
    # Check if there are uncommitted changes
    if git status --porcelain "$service" 2>/dev/null | grep -q .; then
        return 0  # Has uncommitted changes
    fi
    
    # Check build artifacts to see if rebuild is needed
    if [ -f "$service/Cargo.toml" ]; then
        # Rust service - check target directory
        if [ ! -d "$service/target" ]; then
            return 0  # No build artifacts, needs build
        fi
        
        # Check if any source files are newer than the build
        newest_src=$(find "$service/src" -type f -name "*.rs" -printf '%T@\n' 2>/dev/null | sort -rn | head -1 || echo "0")
        if [ -f "$service/target/release/$(basename $service)" ]; then
            build_time=$(stat -c %Y "$service/target/release/$(basename $service)" 2>/dev/null || echo "0")
            if [ "${newest_src%.*}" -gt "$build_time" ]; then
                return 0
            fi
        else
            return 0  # No release build
        fi
    fi
    
    return 1  # No changes detected
}

# Main script
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Get list of services
if [ -n "$1" ]; then
    # Services provided as arguments
    SERVICES="$@"
else
    # Auto-detect services
    SERVICES=$(find_services "$WORKSPACE_ROOT")
fi

# Check each service
changed_services=""
for service in $SERVICES; do
    if has_changed "$service" "$WORKSPACE_ROOT"; then
        changed_services="$changed_services $service"
    fi
done

# Output changed services (trimmed)
echo $changed_services | xargs