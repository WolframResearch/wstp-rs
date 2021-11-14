use wl_expr::{Expr, Number, Symbol};
use wstp::{LinkStr, Protocol, WstpLink};

fn check_loopback_roundtrip(expr: Expr) {
    let mut link = WstpLink::new_loopback().expect("failed to create Loopback link");

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
    let mut link = WstpLink::new_loopback().expect("failed to create Loopback link");

    {
        // Test the `WstpLink::get_string_ref()` method.
        link.put_expr(&Expr::string("Hello!")).unwrap();
        let link_str: LinkStr = link.get_string_ref().unwrap();
        assert_eq!(link_str.to_str(), "Hello!")
    }

    {
        // Test the `WstpLink::get_symbol_ref()` method.
        link.put_expr(&Expr::symbol(Symbol::new("System`Plot").unwrap()))
            .unwrap();
        let link_str: LinkStr = link.get_symbol_ref().unwrap();
        assert_eq!(link_str.to_str(), "System`Plot")
    }
}

#[test]
fn test_is_loopback() {
    let link = WstpLink::new_loopback().unwrap();
    assert!(link.is_loopback());

    let link = WstpLink::listen(Protocol::IntraProcess, "name-test").unwrap();
    assert!(!link.is_loopback());
}
