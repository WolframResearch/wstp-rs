use wl_expr::{Expr, Number, Symbol};
use wstp::{sys, Link, LinkStr, Protocol};

fn check_loopback_roundtrip(expr: Expr) {
    let mut link = Link::new_loopback().expect("failed to create Loopback link");

    link.put_expr(&expr).expect("failed to write expr");

    let read = link.get_expr().expect("failed to read expr");

    assert_eq!(expr, read);
}

#[test]
fn test_loopback_link() {
    check_loopback_roundtrip(Expr::number(Number::Integer(5)));
    check_loopback_roundtrip(Expr::normal(
        Expr::symbol(Symbol::new("System`List").unwrap()),
        vec![Expr::number(Number::Integer(1))],
    ));
    check_loopback_roundtrip(Expr::normal(
        Expr::symbol(Symbol::new("Global`MyHead").unwrap()),
        vec![Expr::number(Number::Integer(1))],
    ));
}

#[test]
fn test_loopback_get_put_atoms() {
    let mut link = Link::new_loopback().expect("failed to create Loopback link");

    {
        // Test the `Link::get_string_ref()` method.
        link.put_expr(&Expr::string("Hello!")).unwrap();
        let link_str: LinkStr = link.get_string_ref().unwrap();
        assert_eq!(link_str.to_str(), "Hello!")
    }

    {
        // Test the `Link::get_symbol_ref()` method.
        link.put_expr(&Expr::symbol(Symbol::new("System`Plot").unwrap()))
            .unwrap();
        let link_str: LinkStr = link.get_symbol_ref().unwrap();
        assert_eq!(link_str.to_str(), "System`Plot")
    }
}

#[test]
fn test_is_loopback() {
    let link = Link::new_loopback().unwrap();
    assert!(link.is_loopback());

    let link = Link::listen(Protocol::IntraProcess, "name-test").unwrap();
    assert!(!link.is_loopback());
}

#[test]
fn test_loopback_idempotence_of_get_arg_count() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };

    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));

    // Test that it doesn't matter how many times we call WSArgCount().
    for _ in 0..5 {
        assert_eq!(link.get_arg_count(), Ok(1));
    }

    assert_eq!(
        link.get_string_ref().map(|s| s.to_str().to_owned()),
        Ok(String::from("List"))
    );
}

#[test]
fn test_get_arg_count_must_be_called_at_least_once() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };

    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));

    // This call is required for this code to be correct:
    //   assert_eq!(link.get_arg_count(), Ok(1));

    // Expect that trying to get the head fails. Even though we know what the arg count
    // should be, we're still required to query it using WSArgCount().
    assert_eq!(
        link.get_string_ref().unwrap_err().code(),
        Some(sys::WSEGSEQ)
    );
}

#[test]
fn test_loopback_basic_put_and_get_list() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };

    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();


    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));
    assert_eq!(link.get_arg_count(), Ok(1));
    assert_eq!(
        link.get_string_ref().map(|s| s.to_str().to_owned()),
        Ok(String::from("List"))
    );
    assert_eq!(link.get_i64(), Ok(10));
}

#[test]
fn test_loopback_get_next() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };

    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();

    assert!(link.is_ready());

    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));
    assert_eq!(link.raw_get_next(), Ok(sys::WSTKSYM.into()));
    assert_eq!(link.raw_get_next(), Ok(sys::WSTKINT.into()));

    assert_eq!(link.raw_get_next().unwrap_err().code(), Some(sys::WSEABORT));

    assert!(!link.is_ready());
}

#[test]
fn test_loopback_new_packet() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };

    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();

    assert!(link.is_ready());

    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));
    assert_eq!(link.new_packet(), Ok(()));

    assert_eq!(link.raw_get_next().unwrap_err().code(), Some(sys::WSEABORT));

    assert!(!link.is_ready());
}

#[test]
fn test_loopback_test_head() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };
    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(link.test_head("List"), Ok(1));
}

#[test]
fn test_loopback_test_head_error() {
    let mut link = Link::new_loopback().unwrap();

    unsafe { sys::WSPutNext(link.raw_link(), sys::WSTKFUNC.into()) };
    link.put_arg_count(1).unwrap();
    link.put_symbol("List").unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(
        link.test_head("Plot").unwrap_err().code().unwrap(),
        sys::WSEGSEQ
    );
}
