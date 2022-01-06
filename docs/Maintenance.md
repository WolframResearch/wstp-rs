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
$ cd wstp-sys
$ cargo make gen-bindings
```

using an appropriate path to the Wolfram product providing the new Wolfram Language
version.