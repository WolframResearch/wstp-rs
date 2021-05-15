use wl_expr::{Expr, Number};
use wl_parse::parse_symbol;
use wl_wstp::{self as wstp, LinkStr, WstpEnv, WstpLink};

fn check_loopback_roundtrip(env: &WstpEnv, expr: Expr) {
    let mut link = WstpLink::new_loopback(&env).expect("failed to create Loopback link");

    link.put_expr(&expr).expect("failed to write expr");

    let read = link.get_expr().expect("failed to read expr");

    assert_eq!(expr, read);
}

#[test]
fn test_loopback_link() {
    let env = wstp::initialize().unwrap();

    check_loopback_roundtrip(&env, Expr::number(Number::Integer(5)));
    check_loopback_roundtrip(
        &env,
        Expr::normal(Expr::symbol(parse_symbol("System`List").unwrap()), vec![
            Expr::number(Number::Integer(1)),
        ]),
    );
    check_loopback_roundtrip(
        &env,
        Expr::normal(Expr::symbol(parse_symbol("Global`MyHead").unwrap()), vec![
            Expr::number(Number::Integer(1)),
        ]),
    );
}

#[test]
fn test_loopback_get_put_atoms() {
    let env = wstp::initialize().unwrap();

    let mut link = WstpLink::new_loopback(&env).expect("failed to create Loopback link");

    {
        // Test the `WstpLink::get_string_ref()` method.
        link.put_expr(&Expr::string("Hello!")).unwrap();
        let link_str: LinkStr = link.get_string_ref().unwrap();
        assert_eq!(link_str.to_str(), "Hello!")
    }

    {
        // Test the `WstpLink::get_symbol_ref()` method.
        link.put_expr(&Expr::symbol(parse_symbol("System`Plot").unwrap()))
            .unwrap();
        let link_str: LinkStr = link.get_symbol_ref().unwrap();
        assert_eq!(link_str.to_str(), "System`Plot")
    }
}
