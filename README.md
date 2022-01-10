# wstp

Bindings to the [Wolfram Symbolic Transfer Protocol (WSTP)](https://www.wolfram.com/wstp/)
library.

This crate provides a set of safe and ergonomic bindings to the WSTP library, used to
transfer Wolfram Language expressions between programs.

## Quick Examples

#### Loopback links

Write an expression to a loopback link, and then read it back from the same link
object:

```rust
use wstp::Link;

fn example() -> Result<(), wstp::Error> {
    let mut link = Link::new_loopback()?;

    // Write the expression {"a", "b", "c"}
    link.put_function("System`List", 3)?;
    link.put_str("a")?;
    link.put_str("b")?;
    link.put_str("c")?;

    // Read back the expression, concatenating the elements as we go:
    let mut buffer = String::new();

    for _ in 0 .. link.test_head("System`List")? {
        buffer.push_str(link.get_string_ref()?.to_str())
    }

    assert_eq!(buffer, "abc");

    Ok(())
}

example();
```

#### Full-duplex links

Transfer the expression `"hello!"` from one [`Link`] endpoint to another:

```rust
use std::{thread, time::Duration};
use wstp::{Link, Protocol};

// Start a background thread with a listen()'ing link.
let listening_thread = thread::spawn(|| {
    // This will block until an incoming connection is made.
    let mut link = Link::listen(Protocol::SharedMemory, "my-link").unwrap();

    link.put_str("hello!").unwrap();
});

// Give the listening thread time to start before we
// try to connect to it.
thread::sleep(Duration::from_millis(20));

let mut link = Link::connect(Protocol::SharedMemory, "my-link").unwrap();
assert_eq!(link.get_string().unwrap(), "hello!");
```

See [`wolfram-library-link`][wolfram-library-link] for
[examples of using WSTP links][wstp-wll-example] to transfer expressions to and from
LibraryLink functions.

[wstp-wll-example]: https://github.com/WolframResearch/wolfram-library-link-rs/blob/master/wolfram-library-link/examples/wstp.rs

## Building `wstp`

The `wstp` crate uses [`wolfram-app-discovery`][wolfram-app-discovery] to locate a local
installation of the Wolfram Language that contains a suitable copy of the WSTP SDK. If the
WSTP SDK cannot be located, `wstp` will fail to build.

If you have installed the Wolfram Language to a location unknown to `wolfram-app-discovery`,
you may specify the installed location manually by setting the `WOLFRAM_APP_DISCOVERY`
environment variable. See [Configuring wolfram-app-discovery] (**TODO**) for details.

## Related Links

#### Related crates

* [`wolfram-library-link`][wolfram-library-link] — author libraries that can be
  dynamically loaded by the Wolfram Language
* [`wolfram-app-discovery`][wolfram-app-discovery] — utility for locating local
  installations of Wolfram applications and the Wolfram Language.


[wolfram-app-discovery]: https://github.com/WolframResearch/wolfram-app-discovery-rs
[wolfram-library-link]: https://github.com/WolframResearch/wolfram-library-link-rs

#### Related documentation

* [WSTP and External Program Communication](https://reference.wolfram.com/language/tutorial/WSTPAndExternalProgramCommunicationOverview.html)
* [How WSTP Is Used](https://reference.wolfram.com/language/tutorial/HowWSTPIsUsed.html)

### Developer Notes

See [**Development.md**](./docs/Development.md) for instructions on how to perform common
development tasks when contributing to the `wstp` crate.

See [**Maintenance.md**](./docs/Maintenance.md) for instructions on how to keep `wstp`
up to date as new versions of the Wolfram Language are released.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Note: Licensing of the WSTP library linked by the [wstp] crate is covered by the terms of
the [MathLink License Agreement](https://www.wolfram.com/legal/agreements/mathlink.html).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for more information.