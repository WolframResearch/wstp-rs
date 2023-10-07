use wstp::Link;

// Test that trying to create a link after shutdown() is called causes an error
// to be returned, instead of a panic.
#[test]
fn test_shutdown() {
    unsafe {
        wstp::shutdown().unwrap();
    }

    assert_eq!(
        Link::new_loopback().unwrap_err().to_string(),
        "WSTP error: wstp-rs: STDENV has been shutdown. No more links can be created."
    );
}
