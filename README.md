# CVM - Crate Version Manager

[![Crates.io](https://img.shields.io/crates/v/cvm_cli.svg)](https://crates.io/crates/cvm_cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A powerful command-line tool for managing semantic versioning of Rust crates in both single-crate and workspace projects.

## Features

- 🚀 **Interactive Version Bumping**: Select crates and bump types (major, minor, patch) interactively
- 📦 **Workspace Support**: Manage multiple crates in a Cargo workspace
- 🔄 **Change Queue System**: Stage version changes and apply them all at once
- 🧪 **Prerelease Mode**: Built-in support for canary/alpha/beta releases with automatic prerelease numbering
- 📝 **Change Summaries**: Require descriptions for all version changes
- 🎯 **Semantic Versioning**: Full compliance with [SemVer](https://semver.org/) specification

## Installation

```bash
cargo install cvm_cli
```

Or build from source:

```bash
git clone https://github.com/lucasaarch/cvm
cd cvm
cargo install --path .
```

## Quick Start

### Initialize CVM in your project

```bash
cd your-rust-project
cvm setup
```

This creates:
- `.cvm/config.toml` - Configuration file
- `.cvm/changes/` - Directory for pending version changes
- `.cvm/README.md` - Information about CVM

### Basic Workflow

1. **Initialize CVM** (first time only):
   ```bash
   cvm setup
   ```

2. **Stage a version change** (interactive mode):
   ```bash
   cvm
   ```
   This will prompt you to select crates and bump types (major, minor, or patch).

3. **Apply all staged changes**:
   ```bash
   cvm apply
   ```

### Prerelease Workflow

1. **Start prerelease mode**:
   ```bash
   cvm pre start canary
   ```
   This saves the current version as the base and enables prerelease mode.

2. **Stage and apply changes** (they will be prerelease versions):
   ```bash
   cvm
   cvm apply
   ```

3. **Exit prerelease mode** when ready for production:
   ```bash
   cvm pre exit
   ```

## How It Works

### Change Queue

When you run `cvm` without arguments, it creates timestamped change files in `.cvm/changes/`. Each file contains:
- A summary of the change
- Which crates to bump (major, minor, or patch)
- Whether it was created in prerelease mode

Running `cvm apply` processes all change files in chronological order and updates the `Cargo.toml` files accordingly.

### Prerelease Mode

Prerelease mode intelligently handles version bumps:

**Example flow starting from `0.1.0`:**

```bash
cvm pre start canary      # Saves base version: 0.1.0
```

Then applying changes sequentially:
- `patch` → `0.1.0-canary.0` (first prerelease of this base)
- `patch` → `0.1.0-canary.1` (increments prerelease number)
- `minor` → `0.2.0-canary.0` (new base: 0.1.0 + minor = 0.2.0)
- `patch` → `0.2.0-canary.1` (same base, increment number)
- `minor` → `0.2.0-canary.2` (base 0.1.0 + minor = 0.2.0, same as current)
- `major` → `1.0.0-canary.0` (new base: 0.1.0 + major = 1.0.0)

**Rules:**
- **PATCH** in prerelease: Always keeps the current base version, increments prerelease number
- **MINOR/MAJOR** in prerelease: Calculates new base from stored base version
  - If different from current base: applies bump and resets to `.0`
  - If same as current base: increments prerelease number

## Commands

### `cvm setup`
Initialize CVM in the current project. Creates configuration files and directories.

**Example:**
```bash
cvm setup
```

**Output:**
```
✅ CVM initialized successfully!

Configuration file created at: .cvm/config.toml
Change files will be stored in: .cvm/changes/

Next steps:
  1. Run 'cvm' to create version changes interactively
  2. Run 'cvm apply' to apply pending changes
  3. Run 'cvm status' to check for pending changes
```

### `cvm` (no arguments)
Interactive mode to select crates and bump types. Creates a change file in `.cvm/changes/`.

**Example:**
```bash
cvm
# Select crates for major bump: [none]
# Select crates for minor bump: [my-crate]
# Select crates for patch bump: [none]
# Summary: Add new feature X
```

### Non-interactive change creation

Create a change file in a single command — ideal for CI, scripts, and bots.

```bash
# Single crate
cvm --crate my-crate --bump patch --summary "Fix scroll clipping"

# Multiple crates, same bump type
cvm --crate foo --crate bar --bump minor --summary "Add TextFieldState API"

# All workspace members
cvm --crate all --bump patch --summary "Dependency pin wgpu 29"

# Per-crate bump types (paired in order)
cvm --crate foo --bump patch --crate bar --bump minor --summary "Mixed bumps"

# Preview without writing (dry run)
cvm --crate my-crate --bump patch --summary "Fix bug" --dry-run
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--crate <NAME>` | Crate to include (repeatable; use `all` for every workspace member) |
| `--bump <TYPE>` | `major`, `minor`, or `patch` — supply once for all crates, or once per crate |
| `--summary <TEXT>` | Required changelog line stored in the change file |
| `--dry-run` | Print the TOML that would be written; do not create the file |

Errors are reported with a non-zero exit code when:
- A crate name is not found in the workspace
- `--bump` count is neither 1 nor equal to the number of `--crate` values
- `--summary` is missing or empty

### `cvm apply`
Applies all pending changes in chronological order.

**Example:**
```bash
cvm apply
# Applying update: Add new feature X
#   my-crate minor → 0.2.0
# All updates applied successfully!
```

**Options:**
- `--dry-run`: Preview changes without applying them

### `cvm status`
Check for pending version changes.

**Example:**
```bash
cvm status
```

Returns exit code 0 if no pending changes, exit code 1 if there are pending changes (useful for CI/CD).

### `cvm info`
Get crate information as JSON. Useful for extracting version numbers and crate metadata in CI/CD pipelines.

**Example:**
```bash
cvm info
# [{"name":"my-crate","version":"1.0.0","path":"Cargo.toml"}]

# Single release version for CI tags (workspaces use [workspace.package].version)
cvm info --version
# 1.0.0
```

**Options:**
- `--version`: Print one release version line (for GitHub Actions tags and scripts)
- `--format <FORMAT>`: Output format (default: json)

### `cvm publish`
Publish crates to crates.io.

**Example:**
```bash
cvm publish
```

**Options:**
- `--dry-run`: Show what would be published without making changes
- `--token <TOKEN>`: Cargo registry token (overrides CARGO_REGISTRY_TOKEN env var)
- `--allow-dirty`: Allow publishing with uncommitted changes

**Output:**
The command outputs JSON at the end with published crate information:
```
::cvm-output-json::[{"name":"my-crate","version":"1.0.0"}]
```
This makes it easy to parse in CI/CD scripts.

### `cvm pre start [identifier]`
Enables prerelease mode with the specified identifier (e.g., `canary`, `alpha`, `beta`, `rc`).

**Examples:**
```bash
cvm pre start canary
cvm pre start alpha
cvm pre start rc
```

### `cvm pre exit`
Disables prerelease mode and clears stored base versions.

**Example:**
```bash
cvm pre exit
# Prerelease mode exited.
```

## File Structure

```
your-project/
├── .cvm/
│   ├── config.toml           # CVM configuration
│   ├── changes/              # Pending version changes
│   │   ├── 1234567890.toml
│   │   └── 1234567891.toml
│   └── README.md             # Auto-generated info
├── Cargo.toml
└── src/
```

### `.cvm/config.toml`

Stores CVM configuration:

```toml
# CVM Configuration

[pre]
enabled = true
identifier = "canary"

[pre.base_versions]
my-crate = "0.1.0"
```

### `.cvm/changes/1234567890.toml`

```toml
[update]
summary = "Add new feature X"
major = []
minor = ["my-crate"]
patch = []
pre = false
```

## Use Cases

### Single Crate Project
Perfect for managing versions of a single library or binary crate.

### Workspace Project
Manage multiple related crates with independent versioning:
- Select which crates to bump
- Different bump types for different crates
- Apply all changes atomically

### Continuous Deployment
1. Developers stage changes with `cvm` as they work
2. CI/CD applies changes with `cvm apply` before publishing
3. Prerelease mode for canary deployments

### Release Management
- Use prerelease mode for beta testing
- Stage multiple changes before release
- Apply all changes at once when ready

## CI/CD Integration

CVM is designed to work seamlessly in CI/CD pipelines. Here's a typical workflow:

### Checking for Pending Changes

Use `cvm status` to check if there are pending version changes:

```bash
cvm status
```

- **Exit code 0**: No pending changes
- **Exit code 1**: There are pending changes (useful for CI conditionals)

### Dry Run

Preview what would be applied without making changes:

```bash
cvm apply --dry-run
```

### Example GitHub Actions Workflow

```yaml
name: Version Management

on:
  push:
    branches: [canary]

jobs:
  apply-versions:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install CVM
        run: cargo install cvm
      
      - name: Check for pending changes
        id: check
        run: |
          if ! cvm status; then
            echo "has_changes=true" >> $GITHUB_OUTPUT
          fi
      
      - name: Apply version changes
        if: steps.check.outputs.has_changes == 'true'
        run: cvm apply
      
      - name: Commit changes
        if: steps.check.outputs.has_changes == 'true'
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Cargo.toml */Cargo.toml
          git commit -m "chore: apply version bumps"
          git push
```

### Typical CI/CD Flow

1. **Development Branch** (`canary`):
   - Developers create changes interactively (`cvm`) or non-interactively
     (`cvm --crate <name> --bump <type> --summary "<text>"`)
   - Commit changes to `.cvm/changes/` directory
   - Push to canary branch

2. **Automated bump in CI** (e.g., after a PR merges):
   ```bash
   cvm --crate my-crate --bump patch --summary "Bump after #42"
   git add .cvm/changes/
   git commit -m "chore: stage patch bump"
   git push
   ```

3. **CI Pipeline Runs**:
   - Check for pending changes: `cvm status`
   - Apply changes: `cvm apply`
   - Run tests with new versions
   - Commit updated `Cargo.toml` files
   - Create PR to main or commit directly

4. **Production Branch** (`main`):
   - Merge PR with version bumps
   - Publish to crates.io: `cvm publish`

### Extracting Version Information in Scripts

Use `cvm info` to extract version information programmatically:

```bash
# Preferred: unified release version (workspace or all crates aligned)
VERSION=$(cvm info --version)
echo "Current version: $VERSION"

# Per-crate JSON (multi-crate workspaces)
VERSION=$(cvm info | jq -r '.[0].version')
echo "Current version: $VERSION"

# Get all crate names and versions
cvm info | jq -r '.[] | "\(.name): \(.version)"'

# Check if a specific crate exists
HAS_CRATE=$(cvm info | jq -r '.[] | select(.name == "my-crate") | .name')
```

When using `cvm publish`, extract the JSON output:

```bash
# Capture publish output
OUTPUT=$(cvm publish 2>&1)

# Extract JSON from output
JSON=$(echo "$OUTPUT" | grep "::cvm-output-json::" | sed 's/.*::cvm-output-json:://')

# Parse published crates
echo "$JSON" | jq -r '.[] | "Published \(.name) v\(.version)"'
```

## Best Practices

1. **Always add meaningful summaries**: They serve as a changelog for your version bumps
2. **Use prerelease mode for testing**: Test breaking changes with canary releases
3. **Commit `.cvm/` to version control**: Share pending changes with your team
4. **Apply changes before publishing**: Run `cvm apply` before `cvm publish`
5. **Use `--dry-run` in CI**: Preview changes before applying them
6. **Check status in pipelines**: Use `cvm status` exit codes for conditional logic

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Authors

- Lucas Arch <luketsx@icloud.com>

## Links

- [Repository](https://github.com/lucasaarch/cvm)
- [Crates.io](https://crates.io/crates/cvm_cli)
- [Documentation](https://docs.rs/cvm_cli)
