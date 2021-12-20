# Development

## Generating `wstp-sys` bindings

`wstp-sys/generated/` contains pre-generated bindings to the WSTP library header file
provided by a particular Wolfram Language on a particular platform. Each time a new
Wolfram Language version is released that makes changes to the WSTP API, the bindings
stored in this crate should be regenerated.

To regenerate the bindings, run the following sequence of commands on each platform that
this crate targets:

```shell
$ export RUST_WOLFRAM_LOCATION=/Applications/Wolfram/Mathematica-12.3.0.app/Contents/
$ cd wstp-sys
$ cargo make gen-bindings
```