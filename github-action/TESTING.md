# Testing the CVM GitHub Action

This guide shows you how to test the CVM GitHub Action before publishing.

## Method 1: Test in a Separate Repository (Recommended)

### Step 1: Create a Test Repository

```bash
# Create a new test repository on GitHub
# For example: your-username/test-cvm-action

# Clone it locally
git clone https://github.com/your-username/test-cvm-action
cd test-cvm-action
```

### Step 2: Create a Simple Rust Project

```bash
# Initialize a Rust project
cargo init --name myapp

# Check initial version
grep version Cargo.toml
# Should show: version = "0.1.0"
```

### Step 3: Create a Test Change File

```bash
# Create CVM change
mkdir -p .cvm/changes

cat > .cvm/changes/test.toml << 'EOF'
[update]
summary = "Test version bump"
major = []
minor = ["myapp"]
patch = []
pre = false
EOF
```

### Step 4: Create Workflow Using Local Action

```bash
mkdir -p .github/workflows

cat > .github/workflows/test-cvm.yml << 'EOF'
name: Test CVM Action

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      # Test using the action from the cvm repository
      - uses: blitzforge/cvm@main/github-action
        with:
          create-pr: false
          token: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Verify version changed
        run: |
          echo "=== Cargo.toml after applying changes ==="
          cat Cargo.toml
          grep 'version = "0.2.0"' Cargo.toml && echo "✅ Version bump successful!"
EOF
```

### Step 5: Commit and Push

```bash
git add .
git commit -m "test: add CVM test workflow"
git push origin main
```

### Step 6: Check Results

1. Go to GitHub Actions tab in your repository
2. Watch the workflow run
3. Check if version was bumped from `0.1.0` to `0.2.0`

## Method 2: Test Locally with the CVM Repository

If you already have the CVM repository cloned:

### Step 1: Create Test Project Inside CVM Repo

```bash
cd /path/to/cvm
mkdir -p test-workspace
cd test-workspace

# Create a test crate
cargo init --name testapp

# Create test change
mkdir -p .cvm/changes
cat > .cvm/changes/1.toml << 'EOF'
[update]
summary = "Test minor bump"
major = []
minor = ["testapp"]
patch = []
pre = false
EOF
```

### Step 2: Test Manually with CVM CLI

```bash
# Install your local CVM
cd /path/to/cvm
cargo install --path .

# Go back to test project
cd test-workspace

# Test dry-run
cvm apply --dry-run

# Test actual apply
cvm apply

# Check result
grep version Cargo.toml
# Should show: version = "0.2.0"
```

## Method 3: Test Action Code Locally

### Option A: Direct Shell Script Test

Extract the shell commands from `action.yml` and run manually:

```bash
cd test-workspace

# Simulate what the action does:

# 1. Install CVM
cargo install cvm

# 2. Check for changes
if cvm status; then
  echo "No changes"
else
  echo "Has changes"
fi

# 3. Apply changes
cvm apply

# 4. Verify
cat Cargo.toml
```

### Option B: Use `act` (GitHub Actions locally)

Install `act`:
```bash
# macOS
brew install act

# Linux
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
```

Then run:
```bash
cd test-workspace
act -j test
```

## Expected Results

### Before Running Action
```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"
```

### After Running Action
```toml
[package]
name = "myapp"
version = "0.2.0"
edition = "2021"
```

### Files Changed
- `Cargo.toml` - version updated
- `.cvm/changes/` - change file removed (applied and deleted)

## Testing Different Scenarios

### Test 1: Dry-Run Mode

```yaml
- uses: blitzforge/cvm@main/github-action
  with:
    dry-run: true
```

Expected: No files changed, preview shown

### Test 2: With PR Creation

```yaml
- uses: blitzforge/cvm@main/github-action
  with:
    create-pr: true
    token: ${{ secrets.GITHUB_TOKEN }}
```

Expected: PR created with version changes

### Test 3: Multiple Changes

Create multiple change files:
```bash
cat > .cvm/changes/1.toml << 'EOF'
[update]
summary = "First change"
major = []
minor = ["myapp"]
patch = []
pre = false
EOF

cat > .cvm/changes/2.toml << 'EOF'
[update]
summary = "Second change"
major = []
minor = []
patch = ["myapp"]
pre = false
EOF
```

Expected: Version goes `0.1.0` → `0.2.0` → `0.2.1`

### Test 4: Prerelease Mode

```bash
# Enable prerelease
cvm pre start canary

# Create change
cat > .cvm/changes/1.toml << 'EOF'
[update]
summary = "Canary release"
major = []
minor = []
patch = ["myapp"]
pre = true
EOF
```

Expected: Version becomes `0.1.0-canary.0`

## Troubleshooting

### "CVM not found"

The action installs CVM automatically. If it fails:
- Check Rust is available in the runner
- Check cargo is in PATH
- Try clearing cache

### "No changes found"

Make sure:
- `.cvm/changes/` directory exists
- `.toml` files are present
- Files are committed to git

### "Version not updated"

Check:
- Crate name in change file matches Cargo.toml
- Cargo.toml is valid TOML
- No errors in action logs

### "PR not created"

Verify:
- `GITHUB_TOKEN` has permissions
- Branch protection allows bot PRs
- Repository settings allow actions to create PRs

## Quick Test Checklist

- [ ] Action installs CVM
- [ ] Detects pending changes
- [ ] Applies version bumps
- [ ] Preserves Cargo.toml formatting
- [ ] Removes change files after apply
- [ ] Creates PR (if enabled)
- [ ] Handles dry-run mode
- [ ] Works with workspaces
- [ ] Works with prerelease mode

## Next Steps

Once testing is complete:
1. Fix any issues found
2. Commit changes to action.yml
3. Follow PUBLISHING_ACTION.md to publish