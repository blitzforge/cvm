#!/bin/bash
set -e

echo "🧪 Testing CVM GitHub Action Locally"
echo "===================================="

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo -e "\n${BLUE}🧹 Cleaning up...${NC}"
    cd ..
    rm -rf test-cvm-action-local
}

# Set trap to cleanup on exit
trap cleanup EXIT

echo -e "\n${BLUE}📁 Creating test project...${NC}"
mkdir -p test-cvm-action-local
cd test-cvm-action-local

echo -e "${BLUE}🦀 Initializing Rust project...${NC}"
cargo init --name testapp --quiet

echo -e "${BLUE}📋 Initial Cargo.toml:${NC}"
cat Cargo.toml | grep -A2 "\[package\]"

echo -e "\n${BLUE}📝 Creating test change file...${NC}"
mkdir -p .cvm/changes

cat > .cvm/changes/1.toml << 'EOF'
[update]
summary = "Test minor version bump"
major = []
minor = ["testapp"]
patch = []
pre = false
EOF

echo "Created change file:"
cat .cvm/changes/1.toml

echo -e "\n${BLUE}🔧 Installing CVM...${NC}"
if ! command -v cvm &> /dev/null; then
    echo "Installing CVM from source..."
    cargo install --path ../
else
    echo "CVM already installed"
fi

echo -e "\n${BLUE}🔍 Checking for pending changes...${NC}"
if cvm status; then
    echo -e "${RED}❌ Expected to find pending changes but found none!${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Pending changes detected${NC}"
fi

echo -e "\n${BLUE}👀 Dry-run test...${NC}"
cvm apply --dry-run

echo -e "\n${BLUE}✨ Applying changes...${NC}"
cvm apply

echo -e "\n${BLUE}📋 Updated Cargo.toml:${NC}"
cat Cargo.toml | grep -A2 "\[package\]"

echo -e "\n${BLUE}🔍 Verifying version...${NC}"
if grep -q 'version = "0.2.0"' Cargo.toml; then
    echo -e "${GREEN}✅ Version successfully updated to 0.2.0${NC}"
else
    echo -e "${RED}❌ Version was not updated correctly!${NC}"
    exit 1
fi

echo -e "\n${BLUE}🔍 Checking if change file was removed...${NC}"
if [ ! -f .cvm/changes/1.toml ]; then
    echo -e "${GREEN}✅ Change file removed after apply${NC}"
else
    echo -e "${RED}❌ Change file still exists!${NC}"
    exit 1
fi

echo -e "\n${BLUE}🧪 Testing prerelease mode...${NC}"
cvm pre start canary

cat > .cvm/changes/2.toml << 'EOF'
[update]
summary = "Test patch in prerelease"
major = []
minor = []
patch = ["testapp"]
pre = true
EOF

echo -e "${BLUE}✨ Applying prerelease change...${NC}"
cvm apply

echo -e "\n${BLUE}📋 Final Cargo.toml:${NC}"
cat Cargo.toml | grep -A2 "\[package\]"

if grep -q 'version = "0.2.0-canary.0"' Cargo.toml; then
    echo -e "${GREEN}✅ Prerelease version correct${NC}"
else
    echo -e "${RED}❌ Prerelease version incorrect!${NC}"
    exit 1
fi

echo -e "\n${GREEN}🎉 All tests passed!${NC}"
echo -e "${GREEN}===================================${NC}"
echo -e "${GREEN}The action logic works correctly.${NC}"
echo -e "${GREEN}Next: Test on GitHub by pushing to a test repo.${NC}"
