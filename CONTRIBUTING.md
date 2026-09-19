# Contributing

Issues and small pull requests are welcome. This project is experimental;
support and review are best effort, without a guaranteed response schedule.
Discuss substantial changes in an issue before starting implementation.

For bug reports, include the revision, toolchain, reproduction steps, expected
result, and actual result. For matching problems, include a minimal scan pair
and matcher configuration when possible. Share only data you can make public.

Keep pull requests focused and explain the behavior they change. Add regression
tests for behavior changes and update relevant documentation.

Run these checks from the repository root:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

For UI changes, also run `npm ci`, `npm run check`, `npm test`, and
`npm run build` in `web/`. The build output in `web/dist` is generated locally
for verification and is not committed; CI rebuilds it from source.

## Preparing a release

Release the library first, then follow the README dependency-upgrade procedure
to pin its selected release commit. Run both Rust and frontend checks. Date the
first public changelog entry on release day. The demo is distributed from source
and versions independently of the library; generate the frontend assets before
packaging or launching a release build.
