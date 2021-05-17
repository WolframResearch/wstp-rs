use wl_wstp::{self as wstp, Protocol, WstpLink};

fn random_link_name() -> String {
    use rand::{distributions::Alphanumeric, Rng};

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}

//======================================
// IntraProcess
//======================================

#[test]
fn test_intra_process_links() {
    // let name: String = dbg!(random_link_name());


    let env = wstp::initialize().unwrap();

    let mut listener = WstpLink::listen(&env, Protocol::IntraProcess, "").unwrap();

    // FIXME: IntraProcess-mode links ignore the `-linkname` device parameter and instead
    //        generate their own random string to use as a name. So we have to create the
    //        listener device first and then ask for it's name.
    let name = listener.link_name();

    let listener_thread = std::thread::spawn(move || {
        listener
            .activate()
            .expect("failed to activate Listener side");

        listener.put_i64(5).unwrap();
        listener.flush().unwrap();

        listener
    });

    let mut connector =
        WstpLink::connect(&env, Protocol::IntraProcess, &name).unwrap();

    let connector_thread = std::thread::spawn(move || {
        connector
            .activate()
            .expect("failed to activate Connector side");


        let got = connector.get_i64().unwrap();
        assert_eq!(got, 5);

        connector
    });

    let _listener = listener_thread.join().unwrap();
    let _connector = connector_thread.join().unwrap();
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
