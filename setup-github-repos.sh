#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}Setting up Syla GitHub repositories...${NC}\n"

# GitHub username
GITHUB_USER="ielm"

# Repository names and paths
REPO_NAMES=("syla-cli" "syla-api-gateway" "syla-execution-service")
REPO_PATHS=("platforms/syla/tools/cli" "platforms/syla/core/api-gateway" "platforms/syla/core/execution-service")

# Function to create GitHub repository
create_github_repo() {
    local repo_name=$1
    echo -e "${BLUE}Creating GitHub repository: ${repo_name}${NC}"
    
    # Check if repo already exists
    if gh repo view "${GITHUB_USER}/${repo_name}" &>/dev/null; then
        echo -e "${GREEN}✓ Repository ${repo_name} already exists${NC}"
    else
        gh repo create "${repo_name}" --public --description "Part of the Syla code execution platform"
        echo -e "${GREEN}✓ Created repository ${repo_name}${NC}"
    fi
}

# Function to setup and push repository
setup_repo() {
    local repo_name=$1
    local repo_path=$2
    
    echo -e "\n${BLUE}Setting up ${repo_name}...${NC}"
    cd "/Users/ivan/Dropbox/Code/datacurve/syla-workspace/${repo_path}"
    
    # Add all files in the repository
    git add .
    
    # Create initial commit if needed
    if ! git log -1 &>/dev/null; then
        git commit -m "Initial commit

🚀 Part of the Syla code execution platform
Repository: ${repo_name}
Platform: Syla"
        echo -e "${GREEN}✓ Created initial commit${NC}"
    else
        echo -e "${GREEN}✓ Repository already has commits${NC}"
    fi
    
    # Set remote origin
    if git remote get-url origin &>/dev/null; then
        echo -e "${GREEN}✓ Remote origin already set${NC}"
    else
        git remote add origin "git@github.com:${GITHUB_USER}/${repo_name}.git"
        echo -e "${GREEN}✓ Added remote origin${NC}"
    fi
    
    # Push to GitHub
    echo -e "${BLUE}Pushing to GitHub...${NC}"
    git push -u origin main
    echo -e "${GREEN}✓ Pushed ${repo_name} to GitHub${NC}"
}

# Main setup process
echo -e "${BLUE}Step 1: Creating GitHub repositories${NC}"
for i in "${!REPO_NAMES[@]}"; do
    create_github_repo "${REPO_NAMES[$i]}"
done

echo -e "\n${BLUE}Step 2: Setting up and pushing repositories${NC}"
for i in "${!REPO_NAMES[@]}"; do
    setup_repo "${REPO_NAMES[$i]}" "${REPO_PATHS[$i]}"
done

# Create parent repository
echo -e "\n${BLUE}Step 3: Creating parent repository${NC}"
cd /Users/ivan/Dropbox/Code/datacurve/syla-workspace

if gh repo view "${GITHUB_USER}/syla" &>/dev/null; then
    echo -e "${GREEN}✓ Parent repository 'syla' already exists${NC}"
else
    gh repo create "syla" --public --description "Syla: A high-performance code execution platform"
    echo -e "${GREEN}✓ Created parent repository 'syla'${NC}"
fi

# Add and commit parent repository files
echo -e "\n${BLUE}Step 4: Setting up parent repository${NC}"
# Add only non-repository files
git add -f .gitignore CLAUDE.md QUICKSTART.md docker-compose.yml syla syla-api-config.toml *.json .cursorrules .eva/ || true
git add .platform/config/ || true
# Try to commit if there are changes
if git diff --cached --quiet; then
    echo -e "${GREEN}✓ No changes to commit in parent repository${NC}"
else
    git commit -m "Initial commit: Syla meta-platform workspace

This is the parent workspace for the Syla code execution platform.
It uses a polyrepo architecture with the following structure:

- platforms/syla/core/api-gateway → github.com/${GITHUB_USER}/syla-api-gateway
- platforms/syla/core/execution-service → github.com/${GITHUB_USER}/syla-execution-service
- platforms/syla/tools/cli → github.com/${GITHUB_USER}/syla-cli

Each subdirectory is its own git repository."

# Set remote for parent
if git remote get-url origin &>/dev/null; then
    echo -e "${GREEN}✓ Parent remote origin already set${NC}"
else
    git remote add origin "git@github.com:${GITHUB_USER}/syla.git"
    echo -e "${GREEN}✓ Added parent remote origin${NC}"
fi

# Push parent repository
git push -u origin main
echo -e "${GREEN}✓ Pushed parent repository to GitHub${NC}"

echo -e "\n${GREEN}✅ All repositories have been set up successfully!${NC}"
echo -e "\nRepositories created:"
echo -e "  - https://github.com/${GITHUB_USER}/syla (parent workspace)"
echo -e "  - https://github.com/${GITHUB_USER}/syla-cli"
echo -e "  - https://github.com/${GITHUB_USER}/syla-api-gateway"
echo -e "  - https://github.com/${GITHUB_USER}/syla-execution-service"