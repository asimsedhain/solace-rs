# Solace-rs

[![crates.io](https://img.shields.io/crates/v/solace-rs.svg)](https://crates.io/crates/solace-rs)


The Unofficial Solace PubSub+ Rust Client Library.
Focuses on providing a safe and idiomatic API over the C Solace library.
It can be used to access the services of a Solace PubSub+ Event Broker.
The C library is not included.


## Features

* Publishing and subscribing
    * Direct
    * Persistent
* Solcache - TODO
* Request Reply - TODO
* Async - TODO

## Installation

```bash
cargo add solace-rs

```

### Configuring Solace Library Link
You can configure the link to use for downloading the solace c library.
Just add the following [configurable-env](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#configurable-env) to your [config.toml file](https://doc.rust-lang.org/cargo/reference/config.html)

```toml
[env]
SOLCLIENT_TARBALL_URL=link-to-c-library-tar-ball

```


## Examples

You can find examples in the [examples folder](./examples). To run them:

```bash
cargo run --example <example_name> -- <example_args>
```
