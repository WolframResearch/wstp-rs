use wstp::{sys, Link, Protocol};

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
fn check_send_data_across_link(mut link_a: Link, mut link_b: Link) {
    let thread_a = std::thread::spawn(move || {
        link_a.activate().expect("failed to activate Listener side");

        // Write an integer.
        link_a.put_i64(5).unwrap();
        link_a.flush().unwrap();

        // Read a f64 written by the other side.
        let got = link_a.get_f64().unwrap();
        assert_eq!(got, 3.1415);

        {
            link_a.put_raw_type(i32::from(sys::WSTKFUNC)).unwrap();
            link_a.put_arg_count(2).unwrap();
            link_a.put_symbol("Sin").unwrap();
            link_a.put_f64(1.0).unwrap();

            link_a.flush().unwrap()
        }

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

        {
            assert_eq!(link_b.get_raw_type(), Ok(i32::from(sys::WSTKFUNC)));
            assert_eq!(link_b.get_arg_count(), Ok(2));
            assert!(link_b.get_symbol_ref().unwrap().to_str() == "Sin");
            assert_eq!(link_b.get_f64(), Ok(1.0))
        }

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

    let listener = Link::listen(Protocol::IntraProcess, "").unwrap();

    // FIXME: IntraProcess-mode links ignore the `-linkname` device parameter and instead
    //        generate their own random string to use as a name. So we have to create the
    //        listener device first and then ask for it's name.
    let name = listener.link_name();

    let connector = Link::connect(Protocol::IntraProcess, &name).unwrap();

    check_send_data_across_link(listener, connector);
}

/// FIXME: IntraProcess-mode links ignore the `-linkname` device parameter and instead
///        generate their own random string to use as a name. So we have to create the
///        listener device first and then ask for it's name.
#[test]
fn test_bug_intra_process_device_ignored_linkname() {
    let name: String = random_link_name();
    let listener = Link::listen(Protocol::IntraProcess, &name).unwrap();
    assert!(name != listener.link_name())
}

//======================================
// SharedMemory
//======================================

/// Test the error code returned by the `SharedMemory` protocol implementation when sync
/// objects with a particular name already exist.
#[test]
fn test_shared_memory_name_taken_error() {
    const NAME: &str = "should-be-taken-2";

    let _a = Link::listen(Protocol::SharedMemory, NAME.into()).unwrap();
    let b = Link::listen(Protocol::SharedMemory, NAME.into());

    assert_eq!(b.unwrap_err().code().unwrap(), sys::MLENAMETAKEN as i32);
}

//======================================
// TCPIP
//======================================

#[test]
fn test_tcpip_links() {
    let listener = Link::listen(Protocol::TCPIP, "8080").unwrap();
    let connector = Link::connect(Protocol::TCPIP, "8080").unwrap();

    check_send_data_across_link(listener, connector);
}

//======================================
// Misc.
//======================================

//-------------------------------------
// Test wait() and wait_with_callback()
//-------------------------------------

#[test]
fn test_link_wait_with_callback() {
    let mut listener = Link::listen(Protocol::IntraProcess, "").unwrap();

    let mut counter = 0;

    listener
        .wait_with_callback(|_: &mut Link| {
            counter += 1;

            if counter < 5 {
                std::ops::ControlFlow::Continue(())
            } else {
                std::ops::ControlFlow::Break(())
            }
        })
        .unwrap();

    assert_eq!(counter, 5);
}

/// Test that `wait_with_callback()` will stop waiting if a panic occurs.
#[test]
fn test_link_wait_with_callback_panic() {
    let mut listener = Link::listen(Protocol::IntraProcess, "").unwrap();

    let mut counter = 0;

    listener
        .wait_with_callback(|_: &mut Link| {
            counter += 1;

            panic!("STOP");
        })
        .unwrap();

    assert_eq!(counter, 1);
}

#[test]
fn test_link_wait_with_callback_drops_closure() {
    use std::sync::Arc;

    let mut listener = Link::listen(Protocol::IntraProcess, "").unwrap();

    let data = Arc::new(());
    let inner: Arc<()> = data.clone();

    assert_eq!(Arc::strong_count(&data), 2);

    // `inner` is moved into `closure`. `inner` will only be dropped if `closure` is. This
    // allows us to indirectly verify that `closure` itself is dropped, even if it panics
    // during the wait. (At a lower level, this is testing an implementation detail of
    // wait_with_callback(): that Box::from_raw(boxed_closure_ptr) is called as expected.)
    let closure = move |_: &mut Link| {
        assert_eq!(Arc::strong_count(&inner), 2);

        panic!()
    };

    listener.wait_with_callback(closure).unwrap();

    assert_eq!(Arc::strong_count(&data), 1);
}

#[test]
fn test_link_wait_with_callback_nested() {
    let mut listener = Link::listen(Protocol::IntraProcess, "").unwrap();

    let mut failed = false;

    listener
        .wait_with_callback(|this: &mut Link| {
            // We're expecting this to panic.
            let _ = this.wait_with_callback(|_| panic!());

            failed = true;
            std::ops::ControlFlow::Break(())
        })
        .unwrap();

    assert!(!failed);
}

//--------------------------------
// Test getting and putting arrays
//--------------------------------

#[test]
fn test_roundtrip_i64_array() {
    let mut link = Link::new_loopback().unwrap();

    link.put_i64_array(&[1, 2, 3, 4], &[2, 2]).unwrap();

    let out = link.get_i64_array().unwrap();

    assert_eq!(out.data().len(), 4);
    assert_eq!(out.dimensions(), &[2, 2]);
}
