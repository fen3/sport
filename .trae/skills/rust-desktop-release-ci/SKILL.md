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
2. **Pin `ubuntu-24.04`, not `ubuntu-latest`,** so the glibc floor stays at 2.39
   and the binary keeps running on newer distros (e.g. glibc 2.41).
3. **ARM64 needs a native runner** (`ubuntu-24.04-arm`); no cross toolchain and
   no extra apt sources required.
4. **Build-time vs run-time deps differ.** `libxkbcommon-dev` etc. are needed to
   compile; the produced binary still requires `libxkbcommon-x11-0`, `libgl1`,
   X11/Wayland client libs on the target machine. Say so in the handoff.
5. **macOS artifact is unsigned and unnotarized.** Users hit Gatekeeper; mention
   `xattr -d com.apple.quarantine ./countdown` if the user complains.

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
  build:
    name: ${{ matrix.artifact_name }}
    runs-on: ${{ matrix.os }}
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

      - name: Install Linux graphics dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libxkbcommon-dev libwayland-dev libx11-dev \
            libxcursor-dev libxi-dev libxrandr-dev libgl1-mesa-dev \
            libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

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
    needs: build
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
- `Swatinem/rust-cache` `workspaces` must point at the directory containing
  `Cargo.toml`, otherwise caching silently misses.
- `ubuntu-24.04-arm` is free for public repos. Private repos require a plan that
  includes ARM runners; check before promising it.
- **Rename binaries before upload.** All four jobs produce a file literally named
  `countdown` / `countdown.exe`; `download-artifact` keeps them in separate
  folders but the Release upload keys on basename, so three of four silently
  overwrite each other and only 2 assets appear. Copy to
  `dist/<artifact_name>[.exe]` and upload that. Use `files: release-assets/**/*`.

## Verification before handing off

```bash
# YAML sanity
python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/build.yml"))'

# Local release build must pass
cd app && cargo build --release --locked

# glibc floor of the Linux binary
objdump -T target/release/app | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1

# Optional GUI smoke test in a headless container
LIBGL_ALWAYS_SOFTWARE=1 timeout 10s xvfb-run -a -s "-screen 0 800x600x24" \
  target/release/app; test $? -eq 124
```

If the smoke test prints `Dynamic loading not supported` you accidentally built
musl/static — rebuild without the musl target.
