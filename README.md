# csm-rs demo

[![CI](https://github.com/LotfiZ/csm-rs-demo/workflows/CI/badge.svg)](https://github.com/LotfiZ/csm-rs-demo/actions/workflows/ci.yml)

An interactive browser demo for [csm-rs](https://github.com/LotfiZ/csm-rs),
a Rust port of Andrea Censi's Canonical Scan Matcher. Generate or import ordered
2D scans, set an initial pose, and inspect the match and its iteration trace.

Experimental, with best-effort support. The demo runs on a local Rust server.

## Requirements

Git and a stable Rust toolchain. Built frontend assets are included; Node.js is
only needed when changing the UI.

## Installation

```sh
git clone https://github.com/LotfiZ/csm-rs-demo.git
cd csm-rs-demo
```

## Configuration

Named runs are saved on the server in `data/experiments`, relative to the working
directory. Set `CSM_DEMO_DATA` to use another directory. Saved runs survive
server restarts; keep that directory to retain them.

## Quick start

```sh
cargo run --locked --release
```

Open <http://127.0.0.1:7878>. To use another local port:

```sh
cargo run --locked --release -- 127.0.0.1:8080
```

## Features

- A Prepare, Place, Match, and Inspect workflow with explicit matching runs.
- Generated asymmetric-room, ambiguous-corridor, and partial-overlap scenarios,
  plus import of ordered polar or Cartesian scan pairs.
- Interactive initial-pose placement: drag to translate and shift-drag to rotate.
- Single-run and iteration-by-iteration matching with correspondence inspection.
- Truth-based translation and rotation errors for generated scans, termination
  reasons, and fixed-reference or previous-frame policies with accumulated drift.
- Named saved runs and replay, including scans and configuration.

Each frame request regenerates its seeded scans and runs the matcher; named
experiments are persisted separately. Imported scans have no ground truth.
Direct sensor connections, ROS integration, and public hosting are out of scope.

## Frontend development

The UI uses Svelte, TypeScript, and Vite under `web/`. Use Node.js 22 and npm,
matching CI. From the repository root:

```sh
cd web
npm ci
npm run check
npm test
npm run build
```

For live reload, run `cargo run` from the repository root in one terminal and
`npm run dev` from `web/` in another. Vite proxies `/api` to `127.0.0.1:7878`.

Commit rebuilt `web/dist` assets alongside frontend changes. CI checks that a
fresh build matches the committed assets.

## Development

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

The demo has its own release cycle. Its library dependency is pinned in
`Cargo.toml`, and `Cargo.lock` records dependency resolution. When upgrading:

1. Update the dependency revision in `Cargo.toml` and `LIBRARY_REVISION` in
   `src/experiments.rs`, which records provenance in saved runs.
2. Run `cargo check` to update `Cargo.lock` and run the development checks above.
3. Include all three files together in the change.

## Changelog and contributions

See [CHANGELOG.md](CHANGELOG.md) for release notes. Issues and small pull requests
are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## Credits and license

The matcher is a Rust port of Andrea Censi's
[Canonical Scan Matcher](https://github.com/AndreaCensi/csm). The demo was
originally developed in [csm-rs](https://github.com/LotfiZ/csm-rs); earlier history
is retained there.

Distributed under **LGPL-3.0-only**. See [LICENSE](LICENSE).
