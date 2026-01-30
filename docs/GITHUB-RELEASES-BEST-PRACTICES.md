# GitHub Releases Best Practices for Desktop Applications

A comprehensive guide to professional GitHub Releases for desktop applications, compiled from current (2025-2026) industry best practices.

## Table of Contents

1. [Release Naming Conventions and Semantic Versioning](#1-release-naming-conventions-and-semantic-versioning)
2. [Release Assets](#2-release-assets-binaries-checksums-signatures)
3. [Release Notes and Changelog](#3-release-notes-format-and-changelog-best-practices)
4. [Pre-releases vs Stable Releases](#4-pre-releases-vs-stable-releases-workflow)
5. [GitHub as Package Distribution](#5-using-github-as-a-package-distribution-channel)
6. [GitHub Pages APT Repository](#6-creating-a-github-pages-apt-repository)
7. [Attestations and Provenance](#7-attestations-and-provenance-for-supply-chain-security)
8. [Examples from Open Source Apps](#8-examples-from-well-known-open-source-desktop-apps)
9. [Tauri-Specific Workflow](#9-tauri-specific-github-release-workflow)

---

## 1. Release Naming Conventions and Semantic Versioning

### Semantic Versioning (SemVer 2.0.0)

Follow the standard `MAJOR.MINOR.PATCH` format:

| Component | When to Increment |
|-----------|-------------------|
| **MAJOR** | Incompatible API/behavior changes, breaking changes |
| **MINOR** | New features added in a backwards-compatible manner |
| **PATCH** | Backwards-compatible bug fixes |

**Pre-release identifiers**: `2.0.0-alpha.1`, `2.0.0-beta.3`, `2.0.0-rc.1`

**Build metadata**: `2.0.0+build.1234` (for informational purposes only)

### Tag Naming Conventions

```
v1.0.0          # Standard version tag
v1.0.0-alpha.1  # Pre-release
v1.0.0-beta.2   # Beta release
v1.0.0-rc.1     # Release candidate
```

### Release Title Format

Good examples:
- `v1.2.0 - Dark Mode Support`
- `v1.2.1 - Bug Fixes`
- `VoKey Transcribe v1.0.0`

### Conventional Commits for Automated Versioning

Using conventional commits enables automated version bumping:

| Commit Type | Version Impact |
|-------------|----------------|
| `fix:` | Patch bump (1.0.0 → 1.0.1) |
| `feat:` | Minor bump (1.0.0 → 1.1.0) |
| `BREAKING CHANGE:` in footer | Major bump (1.0.0 → 2.0.0) |
| `feat!:` or `fix!:` | Major bump |
| `docs:`, `chore:`, `refactor:` | No version bump |

---

## 2. Release Assets (Binaries, Checksums, Signatures)

### Standard Assets to Include

For a cross-platform desktop app:

```
vokey-transcribe_1.0.0_amd64.deb           # Linux Debian/Ubuntu
vokey-transcribe_1.0.0_amd64.AppImage      # Linux universal
vokey-transcribe_1.0.0_x64-setup.exe       # Windows installer
vokey-transcribe_1.0.0_x64_en-US.msi       # Windows MSI
vokey-transcribe_1.0.0_aarch64.dmg         # macOS Apple Silicon
vokey-transcribe_1.0.0_x64.dmg             # macOS Intel
SHA256SUMS.txt                              # Checksums file
SHA256SUMS.txt.asc                          # GPG signature
latest.json                                 # Auto-updater manifest (Tauri)
SBOM.spdx.json                             # Software Bill of Materials
```

### Generating Checksums

```bash
# Generate SHA256 checksums for all release binaries
sha256sum *.deb *.AppImage *.exe *.msi *.dmg > SHA256SUMS.txt

# Or using shasum on macOS
shasum -a 256 *.deb *.AppImage *.exe *.msi *.dmg > SHA256SUMS.txt
```

### GPG Signing Release Assets

```bash
# Create detached signature for the checksums file
gpg --detach-sign --armor SHA256SUMS.txt
# Creates SHA256SUMS.txt.asc

# Sign individual binaries (optional but recommended)
gpg --detach-sign --armor vokey-transcribe_1.0.0_amd64.deb
# Creates vokey-transcribe_1.0.0_amd64.deb.asc
```

### GPG Key Best Practices

- Use RSA 4096-bit keys minimum for future-proofing
- Publish your public key on:
  - Your website
  - Keyservers (keys.openpgp.org)
  - Keybase.io (links to social media for verification)
- Document verification instructions in your README

### Verification Instructions for Users

Include in your release notes:

```markdown
## Verifying Downloads

1. Download the checksums file and signature:
   - `SHA256SUMS.txt`
   - `SHA256SUMS.txt.asc`

2. Import our GPG key:
   ```bash
   gpg --keyserver keys.openpgp.org --recv-keys YOUR_KEY_ID
   ```

3. Verify the signature:
   ```bash
   gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
   ```

4. Verify the checksum:
   ```bash
   sha256sum -c SHA256SUMS.txt
   ```
```

---

## 3. Release Notes Format and Changelog Best Practices

### Keep a Changelog Standard

Use `CHANGELOG.md` in your repository with this structure:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-01-29

### Added
- Dark mode toggle in settings
- Global hotkey customization

### Changed
- Improved transcription accuracy by 15%
- Updated Whisper model to v3

### Fixed
- Memory leak when recording long audio clips
- Crash on startup with certain audio devices

### Removed
- Deprecated legacy audio driver support

## [1.1.0] - 2026-01-15

### Added
- Initial release with core transcription features
```

### GitHub Release Notes Format

```markdown
## What's New in v1.2.0

### Highlights
- **Dark Mode**: Full dark theme support for reduced eye strain
- **Custom Hotkeys**: Configure your preferred global shortcuts

### Added
- Dark mode toggle in Settings (#45)
- Global hotkey customization (#52)

### Changed
- Improved transcription accuracy by 15% (#48)

### Fixed
- Memory leak when recording long audio clips (#41)
- Crash on startup with certain audio devices (#43)

### Security
- Updated dependencies to address CVE-2026-XXXXX

---

**Full Changelog**: https://github.com/username/repo/compare/v1.1.0...v1.2.0

### Checksums

See `SHA256SUMS.txt` for file integrity verification.

### Contributors
Thanks to @contributor1, @contributor2 for their contributions!
```

### Automated Release Notes

Configure `.github/release.yml` for auto-generated notes:

```yaml
changelog:
  exclude:
    labels:
      - ignore-for-release
      - dependencies
    authors:
      - dependabot
  categories:
    - title: "Breaking Changes"
      labels:
        - breaking-change
    - title: "New Features"
      labels:
        - enhancement
        - feature
    - title: "Bug Fixes"
      labels:
        - bug
        - fix
    - title: "Documentation"
      labels:
        - documentation
    - title: "Other Changes"
      labels:
        - "*"
```

---

## 4. Pre-releases vs Stable Releases Workflow

### Release Lifecycle

```
alpha → beta → release candidate (rc) → stable
```

| Stage | Purpose | npm tag | GitHub |
|-------|---------|---------|--------|
| Alpha | Early testing, unstable | `@alpha` | Pre-release |
| Beta | Feature complete, testing | `@beta` | Pre-release |
| RC | Final testing before release | `@rc` | Pre-release |
| Stable | Production ready | `@latest` | Release |

### Branch Strategy for Releases

```
main/master (stable)
├── develop (integration)
│   ├── feat/new-feature
│   └── fix/bug-fix
├── release/v1.2.0 (stabilization)
├── beta (pre-release channel)
└── alpha (early access)
```

### GitHub Actions Workflow for Pre-releases

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Determine release type
        id: release-type
        run: |
          if [[ "${{ github.ref_name }}" == *"-alpha"* ]]; then
            echo "prerelease=true" >> $GITHUB_OUTPUT
            echo "channel=alpha" >> $GITHUB_OUTPUT
          elif [[ "${{ github.ref_name }}" == *"-beta"* ]]; then
            echo "prerelease=true" >> $GITHUB_OUTPUT
            echo "channel=beta" >> $GITHUB_OUTPUT
          elif [[ "${{ github.ref_name }}" == *"-rc"* ]]; then
            echo "prerelease=true" >> $GITHUB_OUTPUT
            echo "channel=rc" >> $GITHUB_OUTPUT
          else
            echo "prerelease=false" >> $GITHUB_OUTPUT
            echo "channel=stable" >> $GITHUB_OUTPUT
          fi

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          prerelease: ${{ steps.release-type.outputs.prerelease }}
          generate_release_notes: true
```

### Semantic Release Configuration for Pre-releases

```json
{
  "branches": [
    "main",
    { "name": "beta", "prerelease": true },
    { "name": "alpha", "prerelease": true }
  ]
}
```

---

## 5. Using GitHub as a Package Distribution Channel

### Direct Download Links

GitHub provides stable URLs for release assets:

```
# Latest release (redirects)
https://github.com/OWNER/REPO/releases/latest/download/ASSET_NAME

# Specific version
https://github.com/OWNER/REPO/releases/download/v1.2.0/ASSET_NAME
```

### Installation Scripts

Provide one-liner installation for users:

```bash
# Example installation script
curl -fsSL https://github.com/owner/repo/releases/latest/download/install.sh | bash
```

### Auto-Update Support (Tauri)

Tauri uses `latest.json` for the updater:

```json
{
  "version": "1.2.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2026-01-29T12:00:00Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "SIGNATURE_CONTENT",
      "url": "https://github.com/owner/repo/releases/download/v1.2.0/app_1.2.0_amd64.AppImage.tar.gz"
    },
    "windows-x86_64": {
      "signature": "SIGNATURE_CONTENT",
      "url": "https://github.com/owner/repo/releases/download/v1.2.0/app_1.2.0_x64-setup.nsis.zip"
    },
    "darwin-aarch64": {
      "signature": "SIGNATURE_CONTENT",
      "url": "https://github.com/owner/repo/releases/download/v1.2.0/app_1.2.0_aarch64.app.tar.gz"
    }
  }
}
```

### GitHub CLI for Downloading

```bash
# Download specific release assets
gh release download v1.2.0 --pattern "*.deb" --repo owner/repo

# Download latest release
gh release download --pattern "*.deb" --repo owner/repo
```

---

## 6. Creating a GitHub Pages APT Repository

### Repository Structure

```
apt-repo/
├── dists/
│   └── stable/
│       ├── main/
│       │   └── binary-amd64/
│       │       ├── Packages
│       │       └── Packages.gz
│       ├── Release
│       ├── Release.gpg
│       └── InRelease
├── pool/
│   └── main/
│       └── v/
│           └── vokey-transcribe/
│               └── vokey-transcribe_1.0.0_amd64.deb
└── KEY.gpg
```

### Setup Script

```bash
#!/bin/bash
# setup-apt-repo.sh

REPO_DIR="apt-repo"
GPG_KEY_ID="YOUR_KEY_ID"

# Create directory structure
mkdir -p $REPO_DIR/dists/stable/main/binary-amd64
mkdir -p $REPO_DIR/pool/main/v/vokey-transcribe

# Copy .deb files
cp *.deb $REPO_DIR/pool/main/v/vokey-transcribe/

# Generate Packages file
cd $REPO_DIR
apt-ftparchive packages pool/ > dists/stable/main/binary-amd64/Packages
gzip -k dists/stable/main/binary-amd64/Packages

# Generate Release file
apt-ftparchive release dists/stable > dists/stable/Release

# Sign Release file
gpg --default-key $GPG_KEY_ID -abs -o dists/stable/Release.gpg dists/stable/Release
gpg --default-key $GPG_KEY_ID --clearsign -o dists/stable/InRelease dists/stable/Release

# Export public key
gpg --armor --export $GPG_KEY_ID > KEY.gpg
```

### GitHub Actions for APT Repository

```yaml
name: Update APT Repository

on:
  release:
    types: [published]

jobs:
  update-apt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          ref: gh-pages

      - name: Download release assets
        run: |
          gh release download ${{ github.event.release.tag_name }} \
            --pattern "*.deb" \
            --dir pool/main/v/vokey-transcribe/
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Import GPG key
        run: |
          echo "${{ secrets.GPG_PRIVATE_KEY }}" | gpg --import

      - name: Update repository
        run: |
          apt-ftparchive packages pool/ > dists/stable/main/binary-amd64/Packages
          gzip -k -f dists/stable/main/binary-amd64/Packages
          apt-ftparchive release dists/stable > dists/stable/Release
          gpg --default-key ${{ secrets.GPG_KEY_ID }} -abs -o dists/stable/Release.gpg dists/stable/Release
          gpg --default-key ${{ secrets.GPG_KEY_ID }} --clearsign -o dists/stable/InRelease dists/stable/Release

      - name: Commit and push
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add -A
          git commit -m "Update APT repository for ${{ github.event.release.tag_name }}"
          git push
```

### User Installation Instructions

```bash
# Add repository GPG key
curl -fsSL https://owner.github.io/repo/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/vokey-archive-keyring.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/vokey-archive-keyring.gpg] https://owner.github.io/repo/apt-repo stable main" | sudo tee /etc/apt/sources.list.d/vokey.list

# Install
sudo apt update
sudo apt install vokey-transcribe
```

---

## 7. Attestations and Provenance for Supply Chain Security

### Overview

GitHub Artifact Attestations provide cryptographic proof of:
- **Where** the software was built (GitHub Actions)
- **How** it was built (workflow details)
- **Who** triggered the build (actor)

This helps achieve **SLSA (Supply-chain Levels for Software Artifacts)** compliance:
- Level 1: Documentation of build process
- Level 2: Tamper-resistant build service (attestations)
- Level 3: Hardened build platform (reusable workflows)

### Adding Attestations to Your Workflow

```yaml
name: Build and Attest

on:
  push:
    tags:
      - 'v*'

permissions:
  id-token: write      # Required for OIDC token
  contents: read       # Required for checkout
  attestations: write  # Required for attestations

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build binary
        run: |
          # Your build commands here
          cargo build --release

      - name: Generate artifact attestation
        uses: actions/attest-build-provenance@v2
        with:
          subject-path: 'target/release/vokey-transcribe'

      - name: Upload to release
        uses: softprops/action-gh-release@v1
        with:
          files: target/release/vokey-transcribe
```

### SBOM Attestation

```yaml
      - name: Generate SBOM
        uses: anchore/sbom-action@v0
        with:
          path: .
          format: spdx-json
          output-file: sbom.spdx.json

      - name: Attest SBOM
        uses: actions/attest-sbom@v1
        with:
          subject-path: 'target/release/vokey-transcribe'
          sbom-path: 'sbom.spdx.json'
```

### Verifying Attestations (User Side)

```bash
# Verify a binary's attestation
gh attestation verify vokey-transcribe_1.0.0_amd64.deb \
  --repo owner/vokey-transcribe

# Verify with JSON output for automation
gh attestation verify vokey-transcribe_1.0.0_amd64.deb \
  --repo owner/vokey-transcribe \
  --format json

# Verify SBOM attestation
gh attestation verify vokey-transcribe_1.0.0_amd64.deb \
  --repo owner/vokey-transcribe \
  --predicate-type https://spdx.dev/Document
```

### Complete Security Workflow

```yaml
name: Secure Release

on:
  push:
    tags:
      - 'v*'

permissions:
  id-token: write
  contents: write
  attestations: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-latest
            target: aarch64-apple-darwin

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Generate SBOM
        uses: anchore/sbom-action@v0
        with:
          format: spdx-json
          output-file: sbom-${{ matrix.target }}.spdx.json

      - name: Attest build provenance
        uses: actions/attest-build-provenance@v2
        with:
          subject-path: 'target/${{ matrix.target }}/release/*'

      - name: Attest SBOM
        uses: actions/attest-sbom@v1
        with:
          subject-path: 'target/${{ matrix.target }}/release/*'
          sbom-path: 'sbom-${{ matrix.target }}.spdx.json'

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/*
```

---

## 8. Examples from Well-Known Open Source Desktop Apps

### Tauri Framework

**Release naming**: Package-prefixed versioning (`tauri v2.9.5`, `tauri-cli v2.9.6`)

**Assets included**:
- Multiple compiled binaries per package
- GPG-verified commits through GitHub's verified signature system

**Release notes features**:
- Cargo Audit section showing security vulnerabilities
- Dependency update notifications
- Contributor acknowledgments

### Signal Desktop

**Distribution**:
- APT repository for Debian/Ubuntu
- Official downloads with checksums
- GPG-signed packages

### VS Code

**Release strategy**:
- Monthly stable releases
- Insiders builds (pre-release channel)
- Platform-specific builds (Windows, macOS, Linux)

**Assets**:
- `.deb`, `.rpm` for Linux
- `.dmg` for macOS (Universal, Intel, Apple Silicon)
- `.exe`, `.msi`, `.zip` for Windows
- `.tar.gz` for portable Linux

### Electron Apps Common Patterns

- Auto-updater integration via Squirrel (Windows/macOS)
- Code signing for all platforms
- Separate release channels (stable, beta, nightly)

---

## 9. Tauri-Specific GitHub Release Workflow

### Complete Tauri Release Workflow

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write
  id-token: write
  attestations: write

jobs:
  create-release:
    runs-on: ubuntu-latest
    outputs:
      release_id: ${{ steps.create-release.outputs.id }}
    steps:
      - uses: actions/checkout@v4

      - name: Create Release
        id: create-release
        uses: softprops/action-gh-release@v1
        with:
          draft: true
          generate_release_notes: true
          prerelease: ${{ contains(github.ref_name, 'alpha') || contains(github.ref_name, 'beta') || contains(github.ref_name, 'rc') }}

  build:
    needs: create-release
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
          - platform: ubuntu-22.04-arm
            target: aarch64-unknown-linux-gnu
          - platform: macos-latest
            target: aarch64-apple-darwin
          - platform: macos-13
            target: x86_64-apple-darwin
          - platform: windows-latest
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Linux dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev librsvg2-dev libasound2-dev

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 'lts/*'
          cache: 'pnpm'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: latest

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Install frontend dependencies
        run: pnpm install

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          releaseId: ${{ needs.create-release.outputs.release_id }}
          args: --target ${{ matrix.target }}

      - name: Generate SBOM
        uses: anchore/sbom-action@v0
        with:
          format: spdx-json
          output-file: sbom-${{ matrix.target }}.spdx.json

      - name: Attest build provenance
        uses: actions/attest-build-provenance@v2
        with:
          subject-path: |
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.deb
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.AppImage
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.dmg
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.exe
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.msi

  publish:
    needs: [create-release, build]
    runs-on: ubuntu-latest
    steps:
      - name: Publish release
        uses: softprops/action-gh-release@v1
        with:
          draft: false
```

### Tauri Updater Configuration

In `tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "pubkey": "YOUR_PUBLIC_KEY",
      "endpoints": [
        "https://github.com/owner/repo/releases/latest/download/latest.json"
      ]
    }
  }
}
```

---

## Quick Reference Checklist

### Before Each Release

- [ ] Update version in `package.json` and `tauri.conf.json`
- [ ] Update `CHANGELOG.md` with release notes
- [ ] Create and push git tag (`git tag -s v1.2.0`)
- [ ] Verify CI builds pass on all platforms

### Release Assets Checklist

- [ ] Linux: `.deb`, `.AppImage` (optional: `.rpm`, `.tar.gz`)
- [ ] Windows: `.exe` (NSIS), `.msi`
- [ ] macOS: `.dmg` (both architectures or Universal)
- [ ] `SHA256SUMS.txt` with checksums
- [ ] `SHA256SUMS.txt.asc` GPG signature
- [ ] `latest.json` for auto-updater
- [ ] SBOM in SPDX or CycloneDX format

### Security Checklist

- [ ] Code signing enabled for Windows/macOS
- [ ] GPG signatures for Linux packages
- [ ] Artifact attestations configured
- [ ] SBOM generated and attested
- [ ] Verification instructions in release notes

---

## Sources

- [Semantic Versioning 2.0.0](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
- [GitHub Artifact Attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations)
- [GitHub Auto-Generated Release Notes](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes)
- [SLSA Framework](https://slsa.dev/)
- [Tauri GitHub Pipelines](https://v2.tauri.app/distribute/pipelines/github/)
- [Debian Wiki: Creating signed GitHub releases](https://wiki.debian.org/Creating%20signed%20GitHub%20releases)
- [CISA SBOM Guidelines 2025](https://www.cisa.gov/resources-tools/resources/2025-minimum-elements-software-bill-materials-sbom)
- [semantic-release](https://github.com/semantic-release/semantic-release)
- [Properly signing GitHub releases](https://gist.github.com/HacKanCuBa/6fabded3565853adebf3dd140e72d33e)
