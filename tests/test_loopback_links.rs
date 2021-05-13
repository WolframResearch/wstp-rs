use wl_expr::{Expr, Number};
use wl_parse::parse_symbol;
use wl_wstp::{self as wstp, WstpLink, WstpEnv};

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
    check_loopback_roundtrip(&env, Expr::normal(
        Expr::symbol(parse_symbol("System`List").unwrap()),
        vec![Expr::number(Number::Integer(1))]
    ));
    check_loopback_roundtrip(&env, Expr::normal(
        Expr::symbol(parse_symbol("Global`MyHead").unwrap()),
        vec![Expr::number(Number::Integer(1))]
    ));
}