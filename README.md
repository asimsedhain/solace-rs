# Solace-rs

[![crates.io](https://img.shields.io/crates/v/solace-rs.svg)](https://crates.io/crates/solace-rs)
[![docs.rs](https://docs.rs/solace-rs/badge.svg)](https://docs.rs/solace-rs/)
[![ci](https://github.com/asimsedhain/solace-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/asimsedhain/solace-rs/actions/workflows/ci.yaml)


The Unofficial Solace PubSub+ Rust Client Library.

Focuses on providing safe and idiomatic rust API over the C Solace library.



## Features

[x] Publishing and subscribing
    [x] Direct
    [x] Persistent
[] Solcache - TODO
[] Request Reply - TODO
[] Async - TODO

## Installation

```bash
cargo add solace-rs

```

### Configuring Solace Library Link
You can configure the url to use for downloading the solace c library.
Just add the following [configurable-env](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#configurable-env) to your [config.toml file](https://doc.rust-lang.org/cargo/reference/config.html)

```toml
[env]
SOLCLIENT_TARBALL_URL=link_to_c_library_tar_ball

```


## Examples

You can find examples in the [examples folder](./examples). To run them:

```bash
cargo run --example <example_name> -- <example_args>
```

## Minimum supported Rust version (MSRV)

The current minimum supported Rust version (MSRV) is 1.66.0.

## OS Support / Test

[x] linux
[x] linux-musl
[x] macos-12
[ ] windows (no plans)

