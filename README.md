# csm-rs demo

A local browser demo that runs the real `csm-rs` matcher on an explicit run. It
ray-casts ordered scans from a moving sensor and draws the reference and sensor
scans with the true and estimated motion.

## Run

Requires a stable Rust toolchain.
Clone it, then run from the repository root:

```sh
git clone https://github.com/LotfiZ/csm-rs-demo.git
cd csm-rs-demo
```

```sh
cargo run --locked --release
```

Then open <http://127.0.0.1:7878>. Pass an address to use another interface or
port:

```sh
cargo run --locked --release -- 0.0.0.0:8080
```

## Features

- A desktop-first workbench organised as Prepare, Place, Match, and Inspect:
  the scan plot stays central while the active step owns the controls.
- Generated previews stay separate from explicit Run. Changing an input clears
  the old result before the new preview arrives, and stale responses cannot
  overwrite newer results.
- Truth-based translation and rotation error, acceptance/termination, and
  ordinary runtime reported separately from instrumented timing.
- Fixed-reference and previous-frame matching policies; the previous-frame
  policy reports accumulated drift.
- Three scenarios (asymmetric room, ambiguous corridor, partial overlap) and a
  matcher settings panel.
- Single-run and step-through modes. Step-through requests one more matcher
  iteration per click and exposes the real iteration trace and correspondences.
- Import of ordered polar/Cartesian scan pairs, named saved runs, and replay.
- Place scan: choose the Place step, drag the sensor scan to move it and
  shift-drag to rotate it, then release to save the pose. Matching stays
  explicit in the Match step. Reset placement returns to the generated guess.

The server is stateless: every `POST /api/frame` regenerates the seeded scans
for the requested step and runs the matcher. Imported data has no ground truth,
so responses show the matcher result only. Direct sensor connections, ROS
integration, and public hosting are out of scope.

## Frontend

The UI is a Vite + Svelte + TypeScript app under `web/`. The built assets in
`web/dist` are committed, so `cargo run` serves a working app without a Node
toolchain. To change the UI:

```sh
cd web
npm install
npm run build
npm run check   # svelte-check
npm test        # vitest: browser-level behaviour jsdom can establish
```

For live reload, run the backend with `cargo run` and start `npm run dev`;
Vite proxies `/api` to `127.0.0.1:7878`.

The workspace uses a four-step workflow rail, collapses to a stacked layout on
narrow screens, and both light and dark themes are defined as design tokens
shared by the plot, legend, and metrics.

The plot uses a neutral interface with restrained colour reserved for scan
identity: reference, sensor, candidate B, and ground-truth scans. Scene boundary
segments are kept as backend context and are not presented as an editable plot
layer. The scene sample control and the matcher iteration stepper are separate:
the former changes generated input, while the latter advances the solver.

## Development

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

This application has its own release cycle. It depends on the
[csm-rs library](https://github.com/LotfiZ/csm-rs) at the Git revision recorded
in `Cargo.toml`; `Cargo.lock` is committed for reproducible dependency resolution.
To upgrade the library, change that revision, run `cargo check` to update the
lockfile, and run the checks above. Commit the manifest and lockfile together.

The initial application was extracted unchanged from `demo/` in csm-rs commit
`5c78370549e3f1cc119fa81a56c2fbe727ca90ff`. Its earlier history remains in that
repository. Interface redesign is separate from this extraction.


## License

LGPL-3.0-only, retained from the source repository. See [LICENSE](LICENSE).
