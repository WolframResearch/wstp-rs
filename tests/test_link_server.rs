use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use wstp::{sys, LinkServer, Protocol, WstpLink};

const PORT: u16 = 11235;

lazy_static::lazy_static! {
    /// Guard used to ensure the [`LinkServer`] tests are run sequentially, so that the
    /// [`PORT`] is free for each test.
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn test_link_server_using_accept() {
    let _guard = MUTEX.lock().unwrap();

    //
    // In a separate thread, spawn a link server to recieve connections.
    //

    let thread = std::thread::spawn(move || {
        let mut server = LinkServer::new(PORT).unwrap();

        assert_eq!(server.port(), Ok(PORT));
        assert!(server.interface().is_ok());

        let mut conn: WstpLink = server
            .accept()
            .expect("failed to wait for link server connection");

        conn.put_i64(0).unwrap();
        conn.flush().unwrap();

        assert_eq!(conn.get_string(), Ok("Done.".to_owned()));

        let before = Instant::now();
        drop(server);
        let after = Instant::now();

        // TODO: Reduce this value the link server close code has been improved to
        //       cancel without a blocking wait.
        assert!(after.duration_since(before) < Duration::from_millis(220));
    });

    // Give the link server time to start listening for connections.
    std::thread::sleep(std::time::Duration::from_millis(100));

    //
    // Create new UUID-based TCPIP connection to the LinkServer connection.
    //

    // Create a connection to the LinkServer, and exchange some data.
    let mut link = WstpLink::connect_with_options(
        Protocol::TCPIP,
        &PORT.to_string(),
        &["MLUseUUIDTCPIPConnection"],
    )
    .unwrap();

    assert_eq!(link.get_i64(), Ok(0));
    link.put_str("Done.").unwrap();
    link.flush().unwrap();

    // Stop the link server.
    thread.join().unwrap();
}

#[test]
fn test_link_server_using_callback() {
    let _guard = MUTEX.lock().unwrap();

    let server = LinkServer::new_with_callback(PORT, |link| {
        println!("Got link: {:?}", link);
    })
    .unwrap();

    // Test that the port and interface getters work as expected.
    assert_eq!(server.port(), Ok(PORT));
    assert!(server.interface().is_ok());
}

#[test]
fn test_name_taken_error() {
    let _guard = MUTEX.lock().unwrap();

    let _a = LinkServer::new_with_callback(PORT, |_| {}).unwrap();
    let b = LinkServer::new_with_callback(PORT, |_| {})
        .expect_err("multiple link servers bound to same port??");

    assert_eq!(b.code(), Some(sys::MLENAMETAKEN as i32));
}
