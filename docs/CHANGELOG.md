# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] – 2022-02-08

Initial release of the [`wstp`](https://crates.io/crates/wstp) crate.

### Added

* [`Link`](https://docs.rs/wstp/0.1.0/wstp/struct.Link.html) struct that represents a
  WSTP link endpoint, and provides methods for reading and writing symbolic Wolfram
  Language expressions.

* [`LinkServer`](https://docs.rs/wstp/0.1.0/wstp/struct.LinkServer.html) struct that
  represents a WSTP TCPIP link server, which binds to a port, listens for incoming
  connections, and creates a new `Link` for each connection.


<!-- This needs to be updated for each tagged release. -->
[Unreleased]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.0...HEAD

[0.1.0]: https://github.com/WolframResearch/wstp-rs/releases/tag/v0.1.0