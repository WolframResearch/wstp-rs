# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] – 2022-05-17

### Fixed

* Fixed wstp-rs build linking issue on Apple Silicon. ([#34])



## [0.2.1] – 2022-03-04

### Fixed

* Fixed documentation build failure in the docs.rs build environment.  ([#32])



## [0.2.0] – 2022-03-03

### Added

* Added Windows support for `wstp` and `wstp-sys`.  ([#29])
  - Add build script commands to link to WSTP interface libraries.
  - Use the [`link-cplusplus`](https://crates.io/crates/link-cplusplus) crate to link to
    the C++ standard library (required by the WSTP library) in a reliable cross-platform
    way.

### Changed

* Changed `wstp-sys` to generate the Rust bindings to `wstp.h` at compile time.  ([#30])

  This ensures that the `wstp` and `wstp-sys` crates will compile with a wider range of
  Wolfram Language versions that provide a suitable version of the WSTP SDK.  See the PR
  description for more info.



## [0.1.4] – 2022-02-19

### Added

* Added [`WolframKernelProcess`](https://docs.rs/wstp/0.1.4/wstp/kernel/struct.WolframKernelProcess.html)
  struct, used to create and manage a WSTP connection to a Wolfram Kernel process.  ([#24])

  `WolframKernelProcess` can be combined with the
  [wolfram-app-discovery](https://crates.io/crates/wolfram-app-discovery) crate to easily
  launch a new Wolfram Kernel session with no manual configuration:

  ```rust
  use std::path::PathBuf;
  use wolfram_app_discovery::WolframApp;
  use wstp::kernel::WolframKernelProcess;

  // Automatically find a local Wolfram Language installation.
  let app = WolframApp::try_default()
      .expect("unable to find any Wolfram Language installations");

  let exe: PathBuf = app.kernel_executable_path().unwrap();

  // Create a new Wolfram Language session using this Kernel.
  let kernel = WolframKernelProcess::launch(&exe).unwrap();
  ```

* Added [`Link::put_eval_packet()`](https://docs.rs/wstp/0.1.4/wstp/struct.Link.html#method.put_eval_packet)
  convenience method to perform evaluations using a connected Wolfram Kernel.  ([#24])

* Added types and methods for ergonomic processing of WSTP tokens.  ([#25])

  A token is the basic unit of expression data that can be read from or written to a link.
  Use the new
  [`Link::get_token()`](https://docs.rs/wstp/0.1.4/wstp/struct.Link.html#method.get_token)
  method to ergonomically match over the
  [`Token`](https://docs.rs/wstp/0.1.4/wstp/enum.Token.html)
  that is readoff of the link:

  ```rust
  use wstp::{Link, Token};

  match link.get_token()? {
      Token::Integer(int) => {
          // Do something with `int`.
      },
      Token::Real(real) => {
          // Do something with `real`.
      },
      ...
  }
  ```

* Added `Link::end_packet()` method.  ([#23])
* Added `wstp::shutdown()`.  ([#23])

### Fixed

* Fixed `Debug` formatting of `LinkStr` to include the string contents.  ([#23])
* Upgrade `wolfram-app-discovery` dependency to v0.2.0 (adds support for app discovery on
  Windows).  ([#25])



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

* [`Link`](https://docs.rs/wstp/0.1.3/wstp/struct.Link.html) struct that represents a
  WSTP link endpoint, and provides methods for reading and writing symbolic Wolfram
  Language expressions.

* [`LinkServer`](https://docs.rs/wstp/0.1.3/wstp/struct.LinkServer.html) struct that
  represents a WSTP TCPIP link server, which binds to a port, listens for incoming
  connections, and creates a new `Link` for each connection.





[#16]: https://github.com/WolframResearch/wstp-rs/pull/16
[#17]: https://github.com/WolframResearch/wstp-rs/pull/17
[#19]: https://github.com/WolframResearch/wstp-rs/pull/19

<!-- v0.1.4 -->
[#23]: https://github.com/WolframResearch/wstp-rs/pull/23
[#24]: https://github.com/WolframResearch/wstp-rs/pull/24
[#25]: https://github.com/WolframResearch/wstp-rs/pull/25

<!-- v0.2.0 -->
[#29]: https://github.com/WolframResearch/wstp-rs/pull/29
[#30]: https://github.com/WolframResearch/wstp-rs/pull/30

<!-- v0.2.1 -->
[#32]: https://github.com/WolframResearch/wstp-rs/pull/32

<!-- v0.2.2 -->
[#34]: https://github.com/WolframResearch/wstp-rs/pull/34


<!-- This needs to be updated for each tagged release. -->
[Unreleased]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.2...HEAD

[0.2.2]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/WolframResearch/wstp-rs/releases/tag/v0.1.0