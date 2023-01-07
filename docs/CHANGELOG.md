# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.6] — 2023-01-06

### Fixed

This release fixes several causes of build failures on Linux.

* Fix use of `i8` instead of `c_char` in variables bound to return values of
  `CStr::from_raw()` and `CString::into_raw()`. ([#45])

  `c_char` is an alias for `i8` on macOS, but it is an alias for `u8` on Linux.

* Fix linker errors by setting missing `-luuid` linker flag in `build.rs`
  on Linux. ([#46])

  `libwstp` depends on the Linux `libuuid` library when targeting Linux.

  *On Ubuntu, `libuuid` is provided by the
  [`uuid-dev` package](https://packages.ubuntu.com/bionic/uuid-dev).*

* Fix broken automatic discovery of `wstp.h` and `libwstp` on Linux by updating
  `wolfram-app-discovery` dependency version. ([#47])



## [0.2.5] — 2023-01-03

### Added

* Add new
  [`wstp::channel()`](https://docs.rs/wstp/0.2.5/wstp/fn.channel.html)
  function, for conveniently creating two connected `Link`s. ([#42])

* Added support for WSTP out-of-band urgent messages. ([#43])

  Add new
  [`UrgentMessage`](https://docs.rs/wstp/0.2.5/wstp/struct.UrgentMessage.html)
  and
  [`UrgentMessageKind`](https://docs.rs/wstp/0.2.5/wstp/enum.UrgentMessageKind.html)
  types.

  Add new `Link` methods:

  - [`Link::is_message_ready()`](https://docs.rs/wstp/0.2.5/wstp/struct.Link.html#method.is_message_ready)
  - [`Link::put_message()`](https://docs.rs/wstp/0.2.5/wstp/struct.Link.html#method.put_message)
  - [`Link::get_message()`](https://docs.rs/wstp/0.2.5/wstp/struct.Link.html#method.get_message)

### Fixed

* Fix issues with `WolframKernelProcess::launch()` ([#42])

  - Fix use of hard-coded linkname. Now a unique linkname is generated automatically.

  - Remove unnecessary background thread, fixing race condition between
    `Link::listen()` in the background thread and the link connection in the
    spawned [`WolframKernel`](https://reference.wolfram.com/language/ref/program/WolframKernel)
    process.

* Fix examples in README.md and the crate root doc comment that exhibit the
  same mistake as `WolframKernelProcess::launch()` bugs mentioned above. ([#42])

### Changed

* Remove redundant attrs on `Link::unchecked_ref_cast_mut()` ([#41])

  *Contributed by dtolnay.*



## [0.2.4] – 2022-10-19

### Fixed

* Fix `` could not find `private` in `ref_cast` `` compile errors that recently
  started occurring to due to changes to semver-exempt private items in the
  `ref-cast` dependency of `wstp`. ([#39])

  Fortunately, the `ref-cast` crate recently gained a `#[ref_cast_custom]`
  macro, which is the missing feature that had originally required `wstp` to
  depend on private internal details of `ref-cast` as a workaround.



## [0.2.3] – 2022-09-19

### Changed

* Mention `get_u8_array()` in the `Array` type doc comment. ([#36])

* Update `wolfram-app-discovery` dependency from 0.2.1 to v0.3.0, to take
  advantage of the improved flexiblity of new API functions tailored for use
  from build scripts. ([#37])



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

<!-- v0.2.3 -->
[#36]: https://github.com/WolframResearch/wstp-rs/pull/36
[#37]: https://github.com/WolframResearch/wstp-rs/pull/37

<!-- v0.2.4 -->
[#39]: https://github.com/WolframResearch/wstp-rs/pull/39

<!-- v0.2.5 -->
[#41]: https://github.com/WolframResearch/wstp-rs/pull/41
[#42]: https://github.com/WolframResearch/wstp-rs/pull/42
[#43]: https://github.com/WolframResearch/wstp-rs/pull/43

<!-- v0.2.5 -->
[#45]: https://github.com/WolframResearch/wstp-rs/pull/45
[#46]: https://github.com/WolframResearch/wstp-rs/pull/46
[#47]: https://github.com/WolframResearch/wstp-rs/pull/47


<!-- This needs to be updated for each tagged release. -->
[Unreleased]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.6...HEAD

[0.2.6]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/WolframResearch/wstp-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/WolframResearch/wstp-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/WolframResearch/wstp-rs/releases/tag/v0.1.0
