# Publishing CVM to crates.io

This guide walks you through publishing CVM to crates.io.

## Prerequisites

1. **crates.io Account**: Create an account at [crates.io](https://crates.io/)
2. **API Token**: Get your API token from [crates.io/me](https://crates.io/me)
3. **Login to cargo**: 
   ```bash
   cargo login <your-api-token>
   ```

## Pre-publication Checklist

- [x] All code compiles without warnings
- [x] README.md exists and is complete
- [x] LICENSE file exists
- [x] Cargo.toml has all required fields:
  - [x] name
  - [x] version
  - [x] authors
  - [x] edition
  - [x] description
  - [x] license
  - [x] repository
  - [x] readme
  - [x] keywords
  - [x] categories

## Steps to Publish

### 1. Commit Your Changes

First, make sure all changes are committed to git:

```bash
git add .
git commit -m "Prepare for v0.1.0 release"
git tag v0.1.0
git push origin main --tags
```

### 2. Verify the Package

Dry-run to see what will be included:

```bash
cargo package --list
```

Build and verify the package:

```bash
cargo package
```

This will:
- Create a `.crate` file in `target/package/`
- Verify that it compiles
- Show any warnings or errors

### 3. Test the Package Locally

Install from the packaged version to test:

```bash
cargo install --path . --force
```

Test the installed binary:

```bash
cvm --help
```

### 4. Publish to crates.io

When everything looks good, publish:

```bash
cargo publish
```

**Note**: You cannot unpublish or modify a published version! Only yank it.

### 5. Verify Publication

After publishing:
1. Check [crates.io/crates/cvm](https://crates.io/crates/cvm)
2. Wait a few minutes for docs to build at [docs.rs/cvm](https://docs.rs/cvm)
3. Test installation from crates.io:
   ```bash
   cargo install cvm
   ```

## Subsequent Releases

For future releases:

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md (create one if needed)
3. Commit changes
4. Tag the release: `git tag v0.2.0`
5. Push: `git push origin main --tags`
6. Publish: `cargo publish`

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Incompatible API changes
- **MINOR** (0.1.0): New functionality, backwards compatible
- **PATCH** (0.0.1): Bug fixes, backwards compatible

## Troubleshooting

### "crate name already exists"
The name `cvm` might be taken. You'll need to:
1. Choose a different name (e.g., `crate-version-manager`)
2. Update `name` in `Cargo.toml`
3. Update references in README.md

### "uncommitted changes"
Use `--allow-dirty` flag:
```bash
cargo publish --allow-dirty
```

### "missing documentation"
Ensure all public items have doc comments:
```rust
/// This function does something important
pub fn my_function() { }
```

## Post-Publication

1. **Announce**: Share on social media, Reddit (/r/rust), etc.
2. **Badge**: Add the crates.io badge to README (already included)
3. **Monitor**: Watch for issues and feedback
4. **Maintain**: Respond to issues and PRs on GitHub

## Useful Commands

```bash
# Check what would be packaged
cargo package --list

# Build the package
cargo package

# Publish (can only be done once per version!)
cargo publish

# Yank a version (doesn't delete, just hides from new projects)
cargo yank --vers 0.1.0

# Un-yank a version
cargo yank --vers 0.1.0 --undo
```

## Additional Resources

- [The Cargo Book - Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [crates.io Policies](https://crates.io/policies)
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)
