# CVM GitHub Action

Automatically manage Rust crate versions in your CI/CD pipeline.

## Usage

### Basic Example

```yaml
name: Version Management

on:
  push:
    branches:
      - canary

jobs:
  version-bump:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: blitzforge/cvm-action@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

That's it! The action will:
1. Check for pending version changes in `.cvm/changes/`
2. Apply them to `Cargo.toml` files
3. Create a PR with the changes

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `command` | What to do: `check-and-apply`, `status`, or `apply` | `check-and-apply` |
| `dry-run` | Preview changes without applying | `false` |
| `create-pr` | Create a pull request | `true` |
| `pr-title` | PR title | `chore: apply version bumps` |
| `pr-labels` | PR labels (comma-separated) | `version-bump,automated` |
| `token` | GitHub token | `${{ github.token }}` |

## Outputs

| Output | Description |
|--------|-------------|
| `has-changes` | `true` if pending changes exist |
| `applied` | `true` if changes were applied |

## Examples

### Dry-Run Mode

Preview what would change:

```yaml
- uses: blitzforge/cvm-action@v1
  with:
    dry-run: true
```

### Custom PR Settings

```yaml
- uses: blitzforge/cvm-action@v1
  with:
    pr-title: "Release: Version updates"
    pr-labels: "release,automated"
```

### Without Creating PR

Apply directly to the branch:

```yaml
- uses: blitzforge/cvm-action@v1
  with:
    create-pr: false

- name: Push changes
  run: |
    git config user.name "github-actions[bot]"
    git config user.email "github-actions[bot]@users.noreply.github.com"
    git add .
    git commit -m "chore: apply version bumps [skip ci]"
    git push
```

### With Tests

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --all

  version-bump:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: blitzforge/cvm-action@v1
```

## How It Works

1. **Developer** creates version changes locally:
   ```bash
   cvm  # Interactive CLI
   git add .cvm/changes/
   git commit -m "feat: add new feature"
   ```

2. **GitHub Action** runs automatically:
   - Detects pending changes
   - Applies version bumps
   - Creates PR for review

3. **Team** reviews and merges the PR

## Requirements

- Rust project with `Cargo.toml`
- CVM change files in `.cvm/changes/`

## Learn More

- [CVM CLI Tool](https://github.com/blitzforge/cvm)
- [CVM on crates.io](https://crates.io/crates/cvm)
- [Report Issues](https://github.com/blitzforge/cvm-action/issues)

## License

MIT