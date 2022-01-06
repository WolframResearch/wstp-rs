# wstp

Bindings to the [Wolfram Symbolic Transfer Protocol (WSTP)](https://www.wolfram.com/wstp/)
library.

This crate provides a set of safe and ergonomic bindings to the WSTP library, used to
transfer Wolfram Language expressions between programs.

# Quick Examples

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

See `wolfram-library-link` for examples of using WSTP links to transfer expressions
to and from LibraryLink functions.

## Related Links

#### Related crates

* `wolfram-library-link` — author libraries that can be dynamically loaded by the Wolfram
  Language

#### Related documentation

* [WSTP and External Program Communication](https://reference.wolfram.com/language/tutorial/WSTPAndExternalProgramCommunicationOverview.html)
* [How WSTP Is Used](https://reference.wolfram.com/language/tutorial/HowWSTPIsUsed.html)

### Developer Notes

See [**Development.md**](./docs/Development.md) for instructions on how to perform common
development tasks when contributing to the `wstp` crate.

See [**Maintenance.md**](./docs/Maintenance.md) for instructions on how to keep `wstp`
up to date as new versions of the Wolfram Language are released.