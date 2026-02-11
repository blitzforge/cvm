# CVM - Crate Version Manager

[![Crates.io](https://img.shields.io/crates/v/cvm.svg)](https://crates.io/crates/cvm)
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
cargo install cvm
```

Or build from source:

```bash
git clone https://github.com/blitzforge/cvm
cd cvm
cargo install --path .
```

## Quick Start

### Basic Workflow

1. **Stage a version change** (interactive mode):
   ```bash
   cvm
   ```
   This will prompt you to select crates and bump types (major, minor, or patch).

2. **Apply all staged changes**:
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

### `cvm apply`
Applies all pending changes in chronological order.

**Example:**
```bash
cvm apply
# Applying update: Add new feature X
#   my-crate minor → 0.2.0
# All updates applied successfully!
```

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

```toml
[pre]
enabled = true
identifier = "canary"

[pre.base_versions]
my-crate = "0.1.0"
another-crate = "1.2.3"
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

See `.github-workflows-example.yml` for more complete examples including:
- Creating PRs for version bumps
- Canary releases with prerelease mode
- Multi-crate workspace handling

### Typical CI/CD Flow

1. **Development Branch** (`canary`):
   - Developers create changes: `cvm` → select crates → add summary
   - Commit changes to `.cvm/changes/` directory
   - Push to canary branch

2. **CI Pipeline Runs**:
   - Check for pending changes: `cvm status`
   - Apply changes: `cvm apply`
   - Run tests with new versions
   - Create PR to main or commit directly

3. **Production Branch** (`main`):
   - Merge PR with version bumps
   - Tag release
   - Publish to crates.io

## Best Practices

1. **Always add meaningful summaries**: They serve as a changelog for your version bumps
2. **Use prerelease mode for testing**: Test breaking changes with canary releases
3. **Commit `.cvm/` to version control**: Share pending changes with your team
4. **Apply changes before publishing**: Run `cvm apply` before `cargo publish`
5. **Use `--dry-run` in CI**: Preview changes before applying them
6. **Check status in pipelines**: Use `cvm status` exit codes for conditional logic

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Authors

- BlitzForge
- Lucas Arch <luketsx@icloud.com>

## Links

- [Repository](https://github.com/blitzforge/cvm)
- [Crates.io](https://crates.io/crates/cvm)
- [Documentation](https://docs.rs/cvm)