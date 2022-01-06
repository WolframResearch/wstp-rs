# Development

This document describes information useful to anyone wishing to modify the `wstp` crate.

## Testing

The `wstp` crate tests should be run using a single testing thread:

```$ shell
cargo test -- --test-threads=1
```

This is necessary to prevent the `LinkServer` tests from all trying to bind to the
same port from multiple threads.

## Override WSTP `CompilerAdditions` location

By default, [build.rs](../build.rs) will use `wolfram-app-discovery` to locate a
local installation of the Wolfram Language that contains a suitable copy of the WSTP
SDK. If you wish to override the WSTP SDK `CompilerAdditions` directory that `wstp` is
linked against, you may set either one of these two environment variables, depending
on your use case:

* `WOLFRAM_APP_DIRECTORY`. Overriding this will force `wolfram-app-discovery` to discover
  this application.
* `WSTP_COMPILER_ADDITIONS`. Overriding this will not change the default app located by
  `wolfram-app-discovery`, but will change the directory linked against in
  [build.rs](../build.rs). This is useful if you have multiple Wolfram products installed,
  or if you are a developer of the WSTP C library.

#### Override examples

Override the `WSTP_COMPILER_ADDITIONS` location:

```shell
$ export WSTP_COMPILER_ADDITIONS=/Applications/Mathematica.app/Contents/SystemFiles/Links/WSTP/DeveloperKit/MacOSX-x86-64/CompilerAdditions
```
