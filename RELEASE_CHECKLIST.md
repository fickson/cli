# Sekuire CLI Release Checklist

Use this checklist when preparing a new CLI release.

---

## Pre-Release (1 week before)

### Version Planning
- [ ] Decide on version number (following [semver](https://semver.org/))
  - Patch (0.1.X): Bug fixes only
  - Minor (0.X.0): New features, backward compatible
  - Major (X.0.0): Breaking changes
- [ ] Create GitHub milestone for the version
- [ ] Review and close all issues in the milestone

### Code Preparation
- [ ] All planned features merged to `main`
- [ ] Run full test suite: `cargo test --all`
- [ ] Run clippy: `cargo clippy --all-targets -- -D warnings`
- [ ] Format code: `cargo fmt --all`
- [ ] Update dependencies: `cargo update`
- [ ] Build all targets locally:
  ```bash
  cargo build --release --target x86_64-unknown-linux-gnu
  cargo build --release --target x86_64-apple-darwin
  cargo build --release --target aarch64-apple-darwin
  ```

### Documentation Updates
- [ ] Update `CHANGELOG.md` with all changes since last release
- [ ] Update `README.md` if usage has changed
- [ ] Update `INSTALL.md` if installation process changed
- [ ] Review and update CLI help text (`--help` output)
- [ ] Update version in `Cargo.toml`:
  ```toml
  [package]
  version = "0.X.0"
  ```

---

## Release Day

### 1. Final Checks
- [ ] Clean workspace: `git status` (should be clean)
- [ ] Pull latest: `git pull origin main`
- [ ] Verify tests pass: `cargo test --all`
- [ ] Smoke test CLI commands:
  ```bash
  sekuire --version
  sekuire init --name test-agent
  sekuire keygen
  sekuire hash
  ```

### 2. Version Bump
- [ ] Update version in `packages/cli/Cargo.toml`
- [ ] Update version in `Cargo.lock`: `cargo update -p sekuire-cli`
- [ ] Commit version bump:
  ```bash
  git add packages/cli/Cargo.toml Cargo.lock
  git commit -m "chore: bump CLI version to v0.X.0"
  git push origin main
  ```

### 3. Create Git Tag
- [ ] Create annotated tag:
  ```bash
  git tag -a v0.X.0 -m "Release v0.X.0"
  ```
- [ ] Push tag (this triggers CI):
  ```bash
  git push origin v0.X.0
  ```

### 4. Monitor CI/CD
- [ ] Go to [GitHub Actions](https://github.com/YOUR_ORG/agent/actions)
- [ ] Watch the `Release Sekuire CLI` workflow
- [ ] Verify all build jobs pass (Linux, macOS Intel, macOS ARM)
- [ ] Check release artifacts are uploaded

### 5. Verify Release
- [ ] Go to [Releases](https://github.com/YOUR_ORG/agent/releases)
- [ ] Verify release is created with tag `v0.X.0`
- [ ] Download and test each binary:
  ```bash
  # macOS ARM
  curl -LO https://github.com/YOUR_ORG/agent/releases/download/v0.X.0/sekuire-darwin-arm64.tar.gz
  tar -xzf sekuire-darwin-arm64.tar.gz
  ./sekuire-darwin-arm64 --version
  
  # Linux
  curl -LO https://github.com/YOUR_ORG/agent/releases/download/v0.X.0/sekuire-linux-amd64.tar.gz
  tar -xzf sekuire-linux-amd64.tar.gz
  ./sekuire-linux-amd64 --version
  ```
- [ ] Verify SHA256 checksums match:
  ```bash
  sha256sum -c sekuire-darwin-arm64.tar.gz.sha256
  ```

---

## Post-Release

### Documentation & Announcements
- [ ] Update [docs.sekuire.ai](https://docs.sekuire.ai) with new version
- [ ] Edit GitHub release notes:
  - Add highlights (new features, breaking changes)
  - Add installation instructions
  - Link to CHANGELOG
  - Add upgrade notes if breaking changes exist
- [ ] Update installation docs with latest version number
- [ ] Announce on:
  - [ ] Discord (`#announcements`)
  - [ ] Twitter/X
  - [ ] Product Hunt (if major release)
  - [ ] Reddit r/rust (if significant Rust-specific improvements)

### Cargo.io Publication (Optional)
- [ ] Publish to crates.io:
  ```bash
  cd packages/cli
  cargo publish --token $CARGO_REGISTRY_TOKEN
  ```
- [ ] Verify package appears on [crates.io/crates/sekuire-cli](https://crates.io/crates/sekuire-cli)

### Homebrew Tap (If applicable)
- [ ] Update Homebrew formula in `homebrew-tap` repo
- [ ] Test installation: `brew install sekuire/tap/sekuire`

### Post-Release Monitoring
- [ ] Monitor GitHub Issues for bug reports related to new release
- [ ] Monitor Discord for user questions/issues
- [ ] Check download stats in GitHub Insights
- [ ] Plan hotfix release if critical bugs found

---

## Rollback Procedure (If needed)

### If Release Has Critical Bug
1. **Delete the release** from GitHub (keep tag)
2. **Create hotfix branch**:
   ```bash
   git checkout v0.X.0
   git checkout -b hotfix-v0.X.1
   ```
3. **Fix the bug** and commit
4. **Follow release process** for v0.X.1
5. **Mark v0.X.0 as "yanked"** in release notes

### If Tag Was Created Incorrectly
```bash
# Delete remote tag
git push origin :refs/tags/v0.X.0

# Delete local tag
git tag -d v0.X.0

# Recreate correctly
git tag -a v0.X.0 -m "Release v0.X.0"
git push origin v0.X.0
```

---

## Version History Template

Keep this updated in `CHANGELOG.md`:

```markdown
## [0.X.0] - 2025-12-XX

### Added
- New `sekuire login` command for authentication
- Support for JWT-based registry authentication

### Changed
- Improved error messages for missing keys
- Updated default registry URL to production endpoint

### Fixed
- Fixed signature verification on Windows
- Resolved OpenSSL linking issues on Linux

### Security
- Updated dependencies to address CVE-XXXX-XXXX
```

---

## Release Cadence

- **Patch releases**: As needed for critical bugs (within 24-48 hours)
- **Minor releases**: Every 2-4 weeks
- **Major releases**: Quarterly or when breaking changes are necessary

---

## Communication Templates

### Discord Announcement
```
🎉 **Sekuire CLI v0.X.0 Released!**

New features:
• Feature 1
• Feature 2

Download: https://github.com/YOUR_ORG/agent/releases/tag/v0.X.0
Changelog: https://github.com/YOUR_ORG/agent/blob/main/packages/cli/CHANGELOG.md

Upgrade: `brew upgrade sekuire` or download the latest binary.
```

### Twitter/X Post
```
🚀 Sekuire CLI v0.X.0 is live!

✨ New: [Key Feature]
🐛 Fixed: [Important Bug]
📦 Download: [link]

#Rust #AI #DevTools
```

---

## Contact

Release Manager: @joel  
Questions: #cli-development channel on Discord
