//! Demonstrates starting a [`LinkServer`] and listening for incoming connections made
//! using [`Link::connect_to_link_server()`].
//!
//! See `examples/connect_to_link_server.rs` for an example of making a connection to this
//! link server. Run both examples in separate terminals to see connections being made.

use wstp::{Link, LinkServer};

const LOCATION: &str = "localhost:55655";

fn main() {
    let server = LinkServer::bind(LOCATION).expect("failed to start LinkServer");

    println!(
        "Started WSTP LinkServer at: {}:{}",
        server.interface(),
        server.port()
    );
    println!("Process ID: {}", std::process::id());

    let mut connection_count = 0;

    for conn in server.incoming() {
        let mut conn: Link = match conn {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("incoming connection error: {}", err);
                continue;
            },
        };

        connection_count += 1;
        println!("got connection #{}", connection_count);

        conn.put_i64(connection_count)
            .expect("failed to write to connection");

        let pid = conn.get_i64().expect("expected link to send PID");
        println!("    connected PID: {}", pid);

        conn.close();
    }
}
