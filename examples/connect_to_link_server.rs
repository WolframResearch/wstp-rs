//! Demonstrates making a connection to a [`LinkServer`] that is listening for incoming
//! connections.
//!
//! See `examples/link_server.rs` for a demonstration of how to create a `LinkServer` that
//! listens for incoming connections.

use wstp::Link;

const LOCATION: &str = "localhost:55655";

fn main() {
    println!("connecting...");

    let mut conn =
        Link::connect_to_link_server(LOCATION).expect("failed to connect to LinkServer");

    println!(
        "Was the {}(st|nd|rd|th) connection!",
        conn.get_i64().unwrap()
    );

    conn.put_i64(std::process::id().into())
        .expect("failed to write PID");

    let _ = conn.raw_next_packet();
}
