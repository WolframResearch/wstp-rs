# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] – 2022-02-08

### Fixed

* Fixed another `wstp-sys` build failure when built in the docs.rs environment.  ([#19])

## [0.1.2] – 2022-02-08

### Fixed

* Fixed `wstp-sys` build failures when built in the docs.rs environment.  ([#17])

## [0.1.1] – 2022-02-08

### Fixed

* Increase `wolfram-app-discovery` dependency version from v0.1.1 to v0.1.2 to get fix
  for [compilation error when compiling for non-macOS targets](https://github.com/WolframResearch/wolfram-app-discovery-rs/blob/master/docs/CHANGELOG.md#012--2022-02-08)
  ([#16])

## [0.1.0] – 2022-02-08

Initial release of the [`wstp`](https://crates.io/crates/wstp) crate.

### Added

* [`Link`](https://docs.rs/wstp/0.1.0/wstp/struct.Link.html) struct that represents a
  WSTP link endpoint, and provides methods for reading and writing symbolic Wolfram
  Language expressions.

* [`LinkServer`](https://docs.rs/wstp/0.1.0/wstp/struct.LinkServer.html) struct that
  represents a WSTP TCPIP link server, which binds to a port, listens for incoming
  connections, and creates a new `Link` for each connection.





[#16]: https://github.com/WolframResearch/wstp-rs/pull/16
[#17]: https://github.com/WolframResearch/wstp-rs/pull/17
[#19]: https://github.com/WolframResearch/wstp-rs/pull/19


<!-- This needs to be updated for each tagged release. -->
[Unreleased]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.3...HEAD

[0.1.3]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/WolframResearch/wstp-rs/releases/tag/v0.1.0