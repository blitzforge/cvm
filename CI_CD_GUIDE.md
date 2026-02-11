# CI/CD Guide for CVM

This guide explains how to use CVM (Crate Version Manager) in your CI/CD pipelines.

## Overview

CVM is designed to work seamlessly in automated workflows. The typical flow is:

1. Developers create version changes locally and commit them
2. CI pipeline detects pending changes
3. CI applies the changes and creates a PR or commits directly
4. Changes are reviewed and merged to production

## Commands for CI/CD

### `cvm status`

Check if there are pending version changes.

**Exit Codes:**
- `0`: No pending changes
- `1`: Pending changes exist

**Example:**
```bash
if ! cvm status; then
  echo "Found pending changes, proceeding with apply..."
fi
```

**Output:**
```
Found 2 pending change(s):

1. Add new authentication feature
   Mode: prerelease
   Minor: ["auth-service", "api-gateway"]

2. Fix memory leak in parser
   Patch: ["parser-lib"]
```

### `cvm apply --dry-run`

Preview what would be applied without making any changes.

**Example:**
```bash
cvm apply --dry-run
```

**Output:**
```
DRY RUN - No changes will be applied

[DRY RUN] Would apply: Add new authentication feature
  (prerelease mode: canary)
  auth-service minor
  api-gateway minor
[DRY RUN] Would apply: Fix memory leak in parser
  parser-lib patch

[DRY RUN] No changes were made.
```

### `cvm apply`

Apply all pending version changes in chronological order.

**Example:**
```bash
cvm apply
```

**Output:**
```
Applying update: Add new authentication feature
  (prerelease mode: canary)
  auth-service minor → 0.2.0-canary.0
  api-gateway minor → 1.3.0-canary.0
Applying update: Fix memory leak in parser
  parser-lib patch → 2.1.1

All updates applied successfully!
```

## GitHub Actions Examples

### Basic Workflow: Apply and Create PR

This workflow runs on the `canary` branch, applies version changes, and creates a PR to `main`.

```yaml
name: Version Management

on:
  push:
    branches:
      - canary

jobs:
  apply-versions:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Install CVM
        run: cargo install cvm

      - name: Check for pending changes
        id: check
        continue-on-error: true
        run: |
          if cvm status; then
            echo "has_changes=false" >> $GITHUB_OUTPUT
            exit 0
          else
            echo "has_changes=true" >> $GITHUB_OUTPUT
            exit 0
          fi

      - name: Show what will be applied
        if: steps.check.outputs.has_changes == 'true'
        run: cvm apply --dry-run

      - name: Apply version changes
        if: steps.check.outputs.has_changes == 'true'
        run: cvm apply

      - name: Configure Git
        if: steps.check.outputs.has_changes == 'true'
        run: |
          git config --global user.name "github-actions[bot]"
          git config --global user.email "github-actions[bot]@users.noreply.github.com"

      - name: Create branch and commit
        if: steps.check.outputs.has_changes == 'true'
        run: |
          BRANCH="version-bump-$(date +%s)"
          git checkout -b "$BRANCH"
          git add Cargo.toml */Cargo.toml .cvm/
          git commit -m "chore: apply version bumps

          Applied all pending version changes from .cvm/changes/"
          git push origin "$BRANCH"
          echo "BRANCH=$BRANCH" >> $GITHUB_ENV

      - name: Create Pull Request
        if: steps.check.outputs.has_changes == 'true'
        uses: peter-evans/create-pull-request@v5
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          branch: ${{ env.BRANCH }}
          base: main
          title: "chore: Version bump from canary"
          body: |
            ## Automated Version Bump

            This PR contains version changes applied from the `canary` branch.

            ### What Changed
            All pending version changes from `.cvm/changes/` have been applied.

            ### Review Checklist
            - [ ] Version numbers look correct
            - [ ] All affected crates are updated
            - [ ] No formatting issues in Cargo.toml files

            ### Next Steps
            - Merge this PR when ready
            - Tag the release if needed
            - Publish to crates.io

            ---
            *Automated by GitHub Actions using CVM*
          labels: |
            version-bump
            automated
```

### Advanced: Conditional Apply with Tests

Run tests before applying changes:

```yaml
name: Version Management with Tests

on:
  push:
    branches:
      - canary

jobs:
  check:
    runs-on: ubuntu-latest
    outputs:
      has_changes: ${{ steps.status.outputs.has_changes }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install CVM
        run: cargo install cvm

      - name: Check status
        id: status
        run: |
          if cvm status; then
            echo "has_changes=false" >> $GITHUB_OUTPUT
          else
            echo "has_changes=true" >> $GITHUB_OUTPUT
          fi

  test:
    needs: check
    if: needs.check.outputs.has_changes == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test --all

      - name: Run clippy
        run: cargo clippy --all -- -D warnings

  apply:
    needs: [check, test]
    if: needs.check.outputs.has_changes == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust & CVM
        run: |
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
          source $HOME/.cargo/env
          cargo install cvm

      - name: Apply changes
        run: cvm apply

      - name: Commit and push
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add .
          git commit -m "chore: apply version bumps [skip ci]"
          git push
```

### Prerelease Workflow

Manage canary/alpha releases:

```yaml
name: Canary Release

on:
  push:
    branches:
      - canary

jobs:
  canary:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install tooling
        run: |
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
          source $HOME/.cargo/env
          cargo install cvm

      - name: Ensure prerelease mode
        run: |
          # Check if prerelease is enabled
          if ! grep -q "enabled = true" .cvm/config.toml 2>/dev/null; then
            cvm pre start canary
            git config user.name "github-actions[bot]"
            git config user.email "github-actions[bot]@users.noreply.github.com"
            git add .cvm/config.toml
            git commit -m "chore: enable canary prerelease mode"
            git push
          fi

      - name: Apply pending changes
        run: |
          if ! cvm status; then
            cvm apply
            git add .
            git commit -m "chore: apply canary version bumps [skip ci]"
            git push
          fi

      - name: Build and test
        run: |
          cargo build --release --all
          cargo test --all

      # Optional: publish canary to crates.io
      - name: Publish canary version
        if: success()
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: |
          for crate in $(find . -name Cargo.toml -not -path "*/target/*"); do
            cd $(dirname $crate)
            cargo publish --allow-dirty || true
            cd -
          done
```

## GitLab CI Example

```yaml
stages:
  - check
  - apply
  - deploy

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo

check-versions:
  stage: check
  image: rust:latest
  script:
    - cargo install cvm
    - cvm status || echo "has_changes=true" >> check.env
  artifacts:
    reports:
      dotenv: check.env

apply-versions:
  stage: apply
  image: rust:latest
  needs:
    - check-versions
  only:
    variables:
      - $has_changes == "true"
  before_script:
    - cargo install cvm
    - git config --global user.name "gitlab-ci"
    - git config --global user.email "gitlab-ci@example.com"
  script:
    - cvm apply --dry-run
    - cvm apply
    - git add Cargo.toml */Cargo.toml .cvm/
    - git commit -m "chore: apply version bumps [skip ci]"
    - git push https://oauth2:${CI_PUSH_TOKEN}@${CI_SERVER_HOST}/${CI_PROJECT_PATH}.git HEAD:${CI_COMMIT_REF_NAME}
```

## Best Practices for CI/CD

### 1. Always Use Dry Run First
```bash
cvm apply --dry-run && cvm apply
```

### 2. Check Exit Codes
```bash
if ! cvm status; then
  # Has pending changes
  cvm apply
fi
```

### 3. Use `[skip ci]` to Avoid Loops
```bash
git commit -m "chore: apply version bumps [skip ci]"
```

### 4. Validate Before Applying
```bash
# Run tests first
cargo test --all

# Then apply
cvm apply
```

### 5. Keep Change Files in Git
Commit `.cvm/changes/` files so the team can see pending changes:
```bash
git add .cvm/changes/*.toml
git commit -m "chore: add version change for feature X"
```

### 6. Clean Up After Merge
After merging to main, the `.cvm/changes/` directory should be empty (files are deleted on apply).

## Troubleshooting

### "No changes found" but I created a change file
- Ensure `.cvm/changes/` exists
- Check file extension is `.toml`
- Verify file is committed to git

### Version not updating in Cargo.toml
- Check crate name matches exactly
- Ensure crate exists in workspace
- Verify Cargo.toml is valid TOML

### Git conflicts in CI
- Use `git pull --rebase` before pushing
- Consider using branch protection rules
- Use PR workflow instead of direct commits

### Changes applied but files still exist
- Ensure `cvm apply` (not `--dry-run`) was used
- Check for errors in the apply step
- Verify write permissions on `.cvm/changes/`

## Environment Variables

CVM doesn't require environment variables, but you may want to set:

```bash
# Skip interactive prompts (for CI)
export CI=true

# Cargo token for publishing
export CARGO_REGISTRY_TOKEN=<your-token>
```

## Security Considerations

1. **Token Permissions**: Use minimal permissions for CI tokens
2. **Branch Protection**: Protect `main` branch, require PR reviews
3. **Audit Trail**: All version changes are tracked in git history
4. **No Secrets**: CVM doesn't use or require secrets (except for publishing)

## Integration with Other Tools

### With Conventional Commits
Parse commit messages to auto-generate change files:
```bash
# Example script
conventional-commits-parser | cvm-generator
```

### With Changelog Generators
Use change summaries to generate changelogs:
```bash
# Extract summaries from .cvm/changes/
for file in .cvm/changes/*.toml; do
  toml get $file update.summary >> CHANGELOG.md
done
```

### With Release Tools
Integrate with `cargo-release` or similar:
```bash
cvm apply
cargo release --execute
```

## Support

For issues or questions:
- GitHub: https://github.com/blitzforge/cvm
- Crates.io: https://crates.io/crates/cvm
- Docs: https://docs.rs/cvm