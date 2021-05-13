use std::sync::Mutex;
use std::net::SocketAddr;

use wstp::{sys, LinkServer};

const PORT: u16 = 11235;

lazy_static::lazy_static! {
    /// Guard used to ensure the [`LinkServer`] tests are run sequentially, so that the
    /// [`PORT`] is free for each test.
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn test_basic_link_server_creation() {
    let _guard = MUTEX.lock().unwrap();

    let env = wstp::initialize().expect("failed to initialize WSTP");

    let _server = LinkServer::new_with_callback(&env, PORT, |link| {
        println!("Got link: {:?}", link);
    })
    .unwrap();
}

#[test]
fn test_name_taken_error() {
    let _guard = MUTEX.lock().unwrap();

    let env = wstp::initialize().expect("failed to initialize WSTP");

    let _a = LinkServer::new_with_callback(&env, PORT, |_| {}).unwrap();
    let b = LinkServer::new_with_callback(&env, PORT, |_| {})
        .expect_err("multiple link servers bound to same port??");

    assert_eq!(b.code(), Some(sys::MLENAMETAKEN as i32));
}