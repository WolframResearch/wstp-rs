# Maintenance

This document describes tasks necessary to maintain the `wstp` and `wstp-sys` crates over
time. This document is informational and intended for the maintainer of these crates;
users of these crates do not need to read this document.

## Generating `wstp-sys` bindings

[`wstp-sys/generated/`](../wstp-sys/generated) contains pre-generated bindings to the
WSTP library header file provided by a particular version of the Wolfram Language on a
particular platform. Each time a new Wolfram Language version is released that makes
changes to the WSTP API, the bindings stored in this crate should be regenerated.

To regenerate the bindings, run the following sequence of commands on each platform that
this crate targets:

```shell
$ export WOLFRAM_APP_DIRECTORY=/Applications/Wolfram/Mathematica-12.3.0.app
$ cargo +nightly xtask gen-bindings
```

using an appropriate path to the Wolfram product providing the new Wolfram Language
version.

### Generating bindings from a particular SDK

```shell
$ cargo +nightly xtask gen-bindings-from ~/Downloads/Linux-ARM64 --target aarch64-unknown-linux-gnu --wolfram-version=13.2.0
```

## Updating build.rs bindings to use on docs.rs

When `wstp-sys` is built in the <docs.rs> environment, some special logic is required
to work around the fact that no Wolfram applications are available to link to.

At the moment, the [`wstp-sys/build.rs`](../wstp-sys/build.rs) file hard-codes a Wolfram
version number and System ID to use as the bindings to display on docs.rs. That version
number should be updated each time new `wstp-sys` bindings are generated.