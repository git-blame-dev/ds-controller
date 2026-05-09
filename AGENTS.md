# Project Notes

Run deterministic project checks with:

```sh
make test
```

The PC receiver has Windows-specific Rust code behind `cfg(windows)`, including the ViGEm backend. When changing PC receiver code, make sure the Windows GitHub Actions job runs, or validate on a Windows machine with:

```sh
cargo test --manifest-path pc/Cargo.toml --locked
cargo clippy --manifest-path pc/Cargo.toml --locked -- -D warnings
```

Do not treat Linux-only Rust tests as complete PC receiver validation because they do not compile every Windows-specific path.
