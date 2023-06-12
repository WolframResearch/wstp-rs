use wolfram_expr::{Expr, Symbol};
use wstp::{sys, Link, LinkStr, Protocol, Token, TokenType};

fn check_loopback_roundtrip(expr: Expr) {
    let mut link = Link::new_loopback().expect("failed to create Loopback link");

    link.put_expr(&expr).expect("failed to write expr");

    let read = link.get_expr().expect("failed to read expr");

    assert_eq!(expr, read);
}

#[test]
fn test_loopback_link() {
    check_loopback_roundtrip(Expr::from(5i64));
    check_loopback_roundtrip(Expr::normal(
        Expr::symbol(Symbol::new("System`List")),
        vec![Expr::from(1i64)],
    ));
    check_loopback_roundtrip(Expr::normal(
        Expr::symbol(Symbol::new("Global`MyHead")),
        vec![Expr::from(1i16)],
    ));
}

#[test]
fn test_loopback_get_put_atoms() {
    let mut link = Link::new_loopback().expect("failed to create Loopback link");

    {
        // Test the `Link::get_string_ref()` method.
        link.put_expr(&Expr::string("Hello!")).unwrap();
        let link_str: LinkStr = link.get_string_ref().unwrap();
        assert_eq!(link_str.as_str(), "Hello!")
    }

    {
        // Test the `Link::get_symbol_ref()` method.
        link.put_expr(&Expr::symbol(Symbol::new("System`Plot")))
            .unwrap();
        let link_str: LinkStr = link.get_symbol_ref().unwrap();
        assert_eq!(link_str.as_str(), "System`Plot")
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

    link.put_function("List", 1).unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));

    // Test that it doesn't matter how many times we call WSArgCount().
    for _ in 0..5 {
        assert_eq!(link.get_arg_count(), Ok(1));
    }

    assert_eq!(
        link.get_string_ref().map(|s| s.as_str().to_owned()),
        Ok(String::from("List"))
    );
}

#[test]
fn test_get_arg_count_must_be_called_at_least_once() {
    let mut link = Link::new_loopback().unwrap();

    link.put_function("List", 1).unwrap();
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

    link.put_function("List", 1).unwrap();
    link.put_i64(10).unwrap();


    assert_eq!(link.raw_get_next(), Ok(sys::WSTKFUNC.into()));
    assert_eq!(link.get_arg_count(), Ok(1));
    assert_eq!(
        link.get_string_ref().map(|s| s.as_str().to_owned()),
        Ok(String::from("List"))
    );
    assert_eq!(link.get_i64(), Ok(10));
}

#[test]
fn test_loopback_get_next() {
    let mut link = Link::new_loopback().unwrap();

    link.put_function("List", 1).unwrap();
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

    link.put_function("List", 1).unwrap();
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

    link.put_function("List", 1).unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(link.test_head("List"), Ok(1));
}

#[test]
fn test_loopback_test_head_error() {
    let mut link = Link::new_loopback().unwrap();

    link.put_function("List", 1).unwrap();
    link.put_i64(10).unwrap();

    assert_eq!(
        link.test_head("Plot").unwrap_err().code().unwrap(),
        sys::WSEGSEQ
    );
}

#[test]
fn test_loopback_transfer_simple() {
    let mut link = Link::new_loopback().unwrap();
    link.put_str("hello").unwrap();

    let mut new = Link::new_loopback().unwrap();
    link.transfer_to_end_of_loopback_link(&mut new).unwrap();

    assert_eq!(new.get_string_ref().unwrap().as_str(), "hello");
}

#[test]
fn test_loopback_transfer_list() {
    let mut link = Link::new_loopback().unwrap();
    link.put_function("System`List", 3).unwrap();
    link.put_i64(5).unwrap();
    link.put_str("second").unwrap();
    link.put_symbol("Global`foo").unwrap();

    let mut new = Link::new_loopback().unwrap();
    link.transfer_to_end_of_loopback_link(&mut new).unwrap();

    assert_eq!(
        new.get_expr().unwrap().to_string(),
        "System`List[5, \"second\", Global`foo]"
    );
}

#[test]
#[rustfmt::skip]
fn test_loopback_get_tokens() {
    // Put {5, "second", foo}
    let mut link = Link::new_loopback().unwrap();
    link.put_function("System`List", 3).unwrap();
    link.put_i64(5).unwrap();
    link.put_str("second").unwrap();
    link.put_symbol("Global`foo").unwrap();

    assert!(matches!(link.get_token().unwrap(), Token::Function { length: 3 }));
    assert!(matches!(link.get_token().unwrap(), Token::Symbol(s) if s.as_str() == "System`List"));
    assert!(matches!(link.get_token().unwrap(), Token::Integer(5)));
    assert!(matches!(link.get_token().unwrap(), Token::String(s) if s.as_str() == "second"));
    assert!(matches!(link.get_token().unwrap(), Token::Symbol(s) if s.as_str() == "Global`foo"));
}

#[test]
#[rustfmt::skip]
fn test_loopback_get_token_type_is_idempotent() {
    // Put {5, "second", foo}
    let mut link = Link::new_loopback().unwrap();
    link.put_function("System`List", 3).unwrap();
    link.put_i64(5).unwrap();
    link.put_str("second").unwrap();
    link.put_symbol("Global`foo").unwrap();

    // Calling get_type(), even multiple times in a row, should not advance the link at
    // all.
    assert_eq!(link.get_type().unwrap(), TokenType::Function);
    assert_eq!(link.get_type().unwrap(), TokenType::Function);
    assert_eq!(link.get_type().unwrap(), TokenType::Function);

    assert!(matches!(link.get_token().unwrap(), Token::Function { length: 3 }));

    assert_eq!(link.get_type().unwrap(), TokenType::Symbol);
    assert_eq!(link.get_type().unwrap(), TokenType::Symbol);
    assert_eq!(link.get_type().unwrap(), TokenType::Symbol);

    assert!(matches!(link.get_token().unwrap(), Token::Symbol(s) if s.as_str() == "System`List"));

    assert_eq!(link.get_type().unwrap(), TokenType::Integer);
    assert_eq!(link.get_type().unwrap(), TokenType::Integer);
    assert_eq!(link.get_type().unwrap(), TokenType::Integer);

    assert!(matches!(link.get_token().unwrap(), Token::Integer(5)));
}
