---
name: "rust-desktop-release-ci"
description: "Multi-platform GitHub Actions build (Linux x64/ARM64, Windows, macOS) plus tag-triggered Release for Rust eframe/egui apps. Invoke when adding CI or producing executables."
---

# Rust Desktop Release CI

Build a Rust desktop app (eframe / egui, or any GUI crate that loads system graphics libraries at runtime) on four targets and publish a GitHub Release from a version tag.

## When to use

- User asks to "配置 GitHub Action 生成可执行文件" for a Rust desktop app
- User wants Windows / macOS / Linux binaries on every push
- User wants `git tag v0.1.0 && git push` to produce a downloadable Release
- User asks for "静态二进制" / "single file binary" on Linux

## Hard constraints (learned from real failures)

1. **Do NOT build eframe/egui with musl or `+crt-static`.**
   - `x86_64-unknown-linux-musl` builds fine but aborts at startup with
     `Dynamic loading not supported` because winit/glow dlopen the GL/X11 libs.
   - `RUSTFLAGS="-C target-feature=+crt-static"` on gnu likewise crashes at startup.
   - A GUI app is never truly static: GL/X11/Wayland must be dlopen'd.
   - If the user insists on "static", explain this and offer either (a) a normal
     dynamic Release, or (b) look into an AppImage-style bundle. Do not silently
     ship an ELF that cannot start.
2. **Linux GLIBC floor** — `ubuntu-24.04` runner produces `GLIBC_2.39` (e.g. `prctl2`,
   `pidfd_*`). To target **GLIBC 2.31 (Ubuntu 20.04)** run the Linux jobs inside
   a `ubuntu:20.04` container on top of the normal runner. Do NOT use
   `rust:1-slim-focal` — that Docker Hub tag was removed when Ubuntu 20.04 hit EOL.
   Install Rust inside the container with `curl https://sh.rustup.rs`
   (`--default-toolchain stable --profile minimal`), not via
   `dtolnay/rust-toolchain@stable` — the action may pull from the host toolchain.
3. **Container gotchas**:
   - `apt-get update` needs `-o Acquire::ForceIPv4=true` or it can stall for
     minutes on GitHub Actions runners with IPv6 misconfig.
   - `DEBIAN_FRONTEND=noninteractive` to avoid debconf prompts.
   - Image is a minimal rootfs: no `curl`, no `rustc`, no `file`/`git`.
     Install `curl`, `ca-certificates`, `build-essential`, `pkg-config`,
     `file`, `git` together with the dev libs.
   - No need for `sudo apt-get` — the container runs as root by default.
   - `Swatinem/rust-cache@v2` cache lives under `$HOME` which differs between
     host and container; either drop it for Linux container jobs or configure
     `cache-on-distro: true`.
4. **ARM64 runner**: `ubuntu-24.04-arm` pulls the arm64 variant of `ubuntu:20.04`
   automatically — no `--platform` flag needed, no cross toolchain.
5. **Verify the result** by downloading the artifact and running
   `objdump -T binary | grep -oE 'GLIBC_[0-9.]+' | sort -Vu` on a Linux dev box.
   For a glibc 2.31 floor the top symbol should be ≤ 2.30 (Rust on 20.04 doesn't
   pull in any 2.31-only routines).
6. **Runner matrix split**: `container:` is a job-level attribute, can't be
   conditionally applied per matrix entry. Split into two jobs: `build_linux`
   (with container) and `build_other` (without). The release job then has
   `needs: [build_linux, build_other]`.
7. **Rename binaries before upload.** All four jobs produce a file literally named
   `countdown` / `countdown.exe`; `download-artifact` keeps them in separate
   folders but the Release upload keys on basename, so three of four silently
   overwrite each other and only 2 assets appear. Copy to
   `dist/<artifact_name>[.exe]` and upload that. Use `files: release-assets/**/*`.
8. **macOS artifact is unsigned.** Users hit Gatekeeper; mention
   `xattr -d com.apple.quarantine ./countdown` if the user complains.
9. **Container platform mismatch** — host glibc version is irrelevant inside a
   container; the linking is entirely done against the container's libc. But
   the build-time linker WILL emit `for GNU/Linux 3.2.0` headers — that's just
   the kernel minimum; the runtime libc floor is what `objdump -T` tells you.

## Workflow template

Place at `<repo>/.github/workflows/build.yml`. Adjust `working-directory` to the
crate path (`countdown` in the reference project).

```yaml
name: Build executables

on:
  push:
    branches:
      - '**'
    tags:
      - 'v*'
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

jobs:
  build_linux:
    name: ${{ matrix.artifact_name }}
    runs-on: ${{ matrix.os }}
    container:
      # ubuntu:20.04 = GLIBC 2.31. EOL but image still available on Docker Hub.
      image: ubuntu:20.04
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-24.04
            artifact_name: app-linux-x86_64
            binary_path: app/target/release/app
          - os: ubuntu-24.04-arm
            artifact_name: app-linux-arm64
            binary_path: app/target/release/app

    steps:
      - name: Check out repository
        uses: actions/checkout@v4

      - name: Install apt dependencies + rustup
        env:
          DEBIAN_FRONTEND: noninteractive
        run: |
          apt-get update -o Acquire::ForceIPv4=true
          apt-get install -y --no-install-recommends \
            curl ca-certificates build-essential pkg-config file git \
            libxkbcommon-dev libwayland-dev libx11-dev \
            libxcursor-dev libxi-dev libxrandr-dev libgl1-mesa-dev \
            libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --default-toolchain stable --profile minimal
          echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"

      - name: Build release executable
        working-directory: app
        run: cargo build --release --locked

      - name: Prepare uniquely named asset
        shell: bash
        run: |
          mkdir -p dist
          cp "${{ matrix.binary_path }}" "dist/${{ matrix.artifact_name }}"

      - name: Upload executable
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact_name }}
          path: dist/*
          if-no-files-found: error

  build_other:
    name: ${{ matrix.artifact_name }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            artifact_name: app-windows-x86_64
            binary_path: app/target/release/app.exe
          - os: macos-latest
            artifact_name: app-macos
            binary_path: app/target/release/app

    steps:
      - name: Check out repository
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: app

      - name: Build release executable
        working-directory: app
        run: cargo build --release --locked

      - name: Prepare uniquely named asset
        shell: bash
        run: |
          mkdir -p dist
          ext=""
          if [ "${{ runner.os }}" = "Windows" ]; then ext=".exe"; fi
          cp "${{ matrix.binary_path }}" "dist/${{ matrix.artifact_name }}${ext}"

      - name: Upload executable
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact_name }}
          path: dist/*
          if-no-files-found: error

  release:
    name: Publish GitHub Release
    needs: [build_linux, build_other]
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - name: Download all executables
        uses: actions/download-artifact@v4
        with:
          path: release-assets

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          name: ${{ github.ref_name }}
          tag_name: ${{ github.ref_name }}
          generate_release_notes: true
          files: release-assets/**/*
```

## Notes on the config

- `contents: read` at workflow level; the release job overrides with
  `contents: write` — minimal privilege, works on public repos with the default
  `GITHUB_TOKEN`, no PAT or secret needed.
- `--locked` fails the build if `Cargo.lock` is stale; intentional, keeps CI
  reproducible. If the repo has no lock file committed, use `--release` only.
- `ubuntu-24.04-arm` is free for public repos. Private repos require a plan that
  includes ARM runners; check before promising it.
- Drop `build_linux` entirely if you don't need the glibc 2.31 floor — just use
  a single `build` job like `build_other` on bare runners for a simpler CI.

## Verification before handing off

```bash
# YAML sanity
python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/build.yml"))'

# If Linux was built in a ubuntu:20.04 container, download the artifact and run
objdump -T countdown-linux-x86_64 | grep -oE 'GLIBC_[0-9.]+' | sort -Vu | tail -1
# Expected top symbol: 2.30 or lower (never 2.31 — Rust on Ubuntu 20.04
# doesn't pull any 2.31-only routines).

# Static check that no NEEDED libs are missing (for non-GUI binaries only)
if readelf -d binary | grep -q '(NEEDED)'; then
  echo "This binary dynamically links system libs — expected for GUI apps."
fi
```

## Known bad ideas (we tried these and they fail)

| Idea | Why it fails |
|---|---|
| `x86_64-unknown-linux-musl` target | Runtime `Dynamic loading not supported` for GL/X11 |
| `RUSTFLAGS=-C target-feature=+crt-static` | Segfault / linker errors at startup |
| `rust:1-slim-focal` container | Docker Hub tag removed (Ubuntu 20.04 EOL) |
| Single job with `container.image: \${{ matrix.container_image \|\| '' }}` | Empty string causes YAML parse / action errors |
| `docker pull rust:1-slim-focal` 404 | Tag removed mid-2025 |
| dtolnay/rust-toolchain inside container | May try to reuse host toolchain paths |
