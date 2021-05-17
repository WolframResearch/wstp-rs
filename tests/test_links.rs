use wstp::{Protocol, WstpLink};

fn random_link_name() -> String {
    use rand::{distributions::Alphanumeric, Rng};

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}

// Helper method to check that data can successfully be sent from `link_a` to `link_b`.
//
// This tests reading and writing from both ends of the link.
fn check_send_data_across_link(mut link_a: WstpLink, mut link_b: WstpLink) {
    let thread_a = std::thread::spawn(move || {
        link_a.activate().expect("failed to activate Listener side");

        // Write an integer.
        link_a.put_i64(5).unwrap();
        link_a.flush().unwrap();

        // Read a f64 written by the other side.
        let got = link_a.get_f64().unwrap();
        assert_eq!(got, 3.1415);

        link_a
    });

    let thread_b = std::thread::spawn(move || {
        link_b
            .activate()
            .expect("failed to activate Connector side");

        let got = link_b.get_i64().unwrap();
        assert_eq!(got, 5);

        link_b.put_f64(3.1415).unwrap();
        link_b.flush().unwrap();

        link_b
    });

    let _link_a = thread_a.join().unwrap();
    let _link_b = thread_b.join().unwrap();
}

//======================================
// IntraProcess
//======================================

#[test]
fn test_intra_process_links() {
    // let name: String = dbg!(random_link_name());


    let env = wstp::initialize().unwrap();

    let listener = WstpLink::listen(&env, Protocol::IntraProcess, "").unwrap();

    // FIXME: IntraProcess-mode links ignore the `-linkname` device parameter and instead
    //        generate their own random string to use as a name. So we have to create the
    //        listener device first and then ask for it's name.
    let name = listener.link_name();

    let connector = WstpLink::connect(&env, Protocol::IntraProcess, &name).unwrap();

    check_send_data_across_link(listener, connector);
}

/// FIXME: IntraProcess-mode links ignore the `-linkname` device parameter and instead
///        generate their own random string to use as a name. So we have to create the
///        listener device first and then ask for it's name.
#[test]
fn test_bug_intra_process_device_ignored_linkname() {
    let env = wstp::initialize().unwrap();

    let name: String = random_link_name();
    let listener = WstpLink::listen(&env, Protocol::IntraProcess, &name).unwrap();
    assert!(name != listener.link_name())
}

//======================================
// TCPIP
//======================================

#[test]
fn test_tcpip_links() {
    let env = wstp::initialize().unwrap();

    let listener = WstpLink::listen(&env, Protocol::TCPIP, "8080").unwrap();
    let connector = WstpLink::connect(&env, Protocol::TCPIP, "8080").unwrap();

    check_send_data_across_link(listener, connector);
}
