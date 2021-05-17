# wl-wstp

See [wl-library-link][wl-library-link] for examples of using this library in LibraryLink
functions.

[wl-library-link]: https://stash.wolfram.com/users/connorg/repos/wl-library-link/browse

## Reading the documentation

This will generate source code documentation and open it in your web browser.

```shell
$ cargo doc --document-private-items --open
```

## Development

#### `WSTP_COMPILER_ADDITIONS`

By default, the `wl-wstp-sys/build.rs` script will attempt to use
[`wolframscript`](https://www.wolfram.com/wolframscript/) to evaluate
[`$InstallationDirectory`](https://reference.wolfram.com/language/ref/$InstallationDirectory.html)
to locate your local Wolfram installation, and will use the WSTP library version contained
within the application contents.

The [`WSTP_COMPILER_ADDITIONS`] environment variable can be used manual specify the WSTP
library location to use. This is useful if you have multiple Wolfram products installed,
or if you are a developer of the WSTP library.

```shell
$ export WSTP_COMPILER_ADDITIONS=/Applications/Mathematica.app/Contents/SystemFiles/Links/WSTP/DeveloperKit/MacOSX-x86-64/CompilerAdditions
```
