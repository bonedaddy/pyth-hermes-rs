# Pyth Hermes Rust Client

[![Crates.io](https://img.shields.io/crates/v/pyth-hermes-rs.svg)](https://crates.io/crates/pyth-hermes-rs)
[![Documentation](https://docs.rs/pyth-hermes-rs/badge.svg)](https://docs.rs/pyth-hermes-rs)

Rust library providing a HTTP client for querying the [Pyth Hermes API](https://hermes.pyth.network/docs/#/). Supports all non deprecated API calls

## SSE Support

`pyth-hermes-rs` supports SSE price updates, allowing you to receive updates in real time without having to poll the API