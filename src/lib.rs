use std::convert::TryFrom;
use std::ffi::CStr;

use wl_expr::{Expr, Number, Symbol};
use wl_wstp_sys::{
    WSErrorMessage, WSGetArgCount, WSGetInteger64, WSGetReal64, WSGetSymbol, WSGetType,
    WSGetUTF8String, WSReleaseErrorMessage, WSReleaseString, WSReleaseSymbol, WSLINK,
};

pub struct WSTPLink {
    link: WSLINK,
}

impl WSTPLink {
    pub unsafe fn new(link: WSLINK) -> Self {
        WSTPLink { link }
    }

    pub fn get_expr(&self) -> Result<Expr, String> {
        let WSTPLink { link } = *self;

        unsafe { get_expr(link) }
    }
}

unsafe fn error_message(link: WSLINK) -> Option<String> {
    let message: *const i8 = WSErrorMessage(link);

    if message.is_null() {
        return None;
    }

    let cstr = CStr::from_ptr(message);
    let string: String = cstr.to_str().unwrap().to_owned();

    WSReleaseErrorMessage(link, message);

    return Some(string);
}

unsafe fn error_message_or_unknown(link: WSLINK) -> String {
    error_message(link).unwrap_or_else(|| "unknown error occurred on WSLINK".into())
}

//======================================
// Read from the link
//======================================

unsafe fn get_expr(link: WSLINK) -> Result<Expr, String> {
    use wl_wstp_sys::{WSTKERR, WSTKFUNC, WSTKINT, WSTKREAL, WSTKSTR, WSTKSYM};

    let type_: i32 = WSGetType(link);

    if type_ == WSTKERR as i32 {
        return Err(error_message_or_unknown(link));
    }

    let expr: Expr = match type_ as u8 {
        WSTKINT => {
            let mut int = 0;
            if WSGetInteger64(link, &mut int) == 0 {
                return Err(error_message_or_unknown(link));
            }
            Expr::number(Number::Integer(int))
        },
        WSTKREAL => {
            let mut real: f64 = 0.0;
            if WSGetReal64(link, &mut real) == 0 {
                return Err(error_message_or_unknown(link));
            }
            let real: wl_expr::F64 = match wl_expr::F64::new(real) {
                Ok(real) => real,
                // TODO: Try passing a NaN value or a BigReal value through WSLINK.
                Err(_is_nan) => {
                    return Err(format!(
                        "NaN value passed on WSLINK cannot be used to construct an Expr"
                    ))
                },
            };
            Expr::number(Number::Real(real))
        },
        WSTKSTR => {
            let mut c_string: *const u8 = std::ptr::null();
            let mut num_bytes: i32 = 0;
            let mut num_chars = 0;
            if WSGetUTF8String(link, &mut c_string, &mut num_bytes, &mut num_chars) == 0 {
                // NOTE: According to the documentation, we do NOT have to release
                //      `string` if the function returns an error.
                return Err(error_message_or_unknown(link));
            }

            let string = copy_and_release_cstring(
                link,
                c_string,
                usize::try_from(num_bytes).unwrap(),
                false,
            );

            Expr::string(string)
        },
        WSTKSYM => {
            let mut c_string: *const i8 = std::ptr::null();

            if WSGetSymbol(link, &mut c_string) == 0 {
                return Err(error_message_or_unknown(link));
            }

            let string: String = {
                let cstr = CStr::from_ptr(c_string);

                let string: &str = cstr.to_str().unwrap();
                string.to_owned()
            };

            WSReleaseString(link, c_string);

            let symbol: Symbol = match wl_parse::parse_symbol(&string) {
                Some(sym) => sym,
                None => return Err(format!("Symbol name `{}` has no context", string)),
            };

            Expr::symbol(symbol)
        },
        WSTKFUNC => {
            let mut arg_count = 0;

            if WSGetArgCount(link, &mut arg_count) == 0 {
                return Err(error_message_or_unknown(link));
            }

            let arg_count = usize::try_from(arg_count)
                .expect("WSTKFUNC argument count could not be converted to usize");

            let head = get_expr(link)?;

            let mut contents = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                contents.push(get_expr(link)?);
            }

            Expr::normal(head, contents)
        },
        _ => return Err(format!("unknown WSLINK type: {}", type_)),
    };

    Ok(expr)
}

//======================================
// Utilities
//======================================

/// This function will panic if `c_string` is not valid UTF-8.
unsafe fn copy_and_release_cstring(
    link: WSLINK,
    c_string: *const u8,
    byte_count: usize,
    is_symbol: bool,
) -> String {
    let bytes: &[u8] = std::slice::from_raw_parts(c_string, byte_count);

    let string: String = match String::from_utf8(bytes.to_vec()) {
        Ok(string) => string,
        Err(_) => String::from_utf8_lossy(bytes).to_string(),
    };

    let c_string = c_string as *const i8;

    // Deallocate the string data.
    match is_symbol {
        // TODO: It's not clear if there is actually any difference between
        //       WSReleaseSymbol() and WSReleaseString(). It's probable that they're both
        //       implemented by just calling free(). Verify this and remove this branch
        //       and the `is_symbol` parameter.
        true => WSReleaseSymbol(link, c_string),
        false => WSReleaseString(link, c_string),
    }

    string
}
