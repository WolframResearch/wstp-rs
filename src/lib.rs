mod error;

use std::convert::TryFrom;
use std::ffi::{CStr, CString};

use wl_expr::{Expr, ExprKind, Normal, Number, Symbol};
use wl_wstp_sys::{
    WSClearError, WSEndPacket, WSErrorMessage, WSGetArgCount, WSGetInteger64,
    WSGetReal64, WSGetSymbol, WSGetType, WSGetUTF8String, WSNewPacket, WSPutArgCount,
    WSPutInteger64, WSPutReal64, WSPutType, WSPutUTF8String, WSPutUTF8Symbol, WSReady,
    WSReleaseErrorMessage, WSReleaseString, WSReleaseSymbol, WSLINK,
};

//-----------------------------------
// Public re-exports and type aliases
//-----------------------------------

pub use crate::error::Error;
pub use wl_wstp_sys as sys;

// TODO: Remove this type alias after outside code has had time to update.
#[deprecated(note = "use WstpLink")]
pub type WSTPLink = WstpLink;

//======================================
// Source
//======================================

macro_rules! link_try {
    ($link:expr, $op:expr) => {{
        let link: WSLINK = $link;
        if $op == 0 {
            return Err(error_message_or_unknown(link));
        }
    }};
}

/// A WSTP library environment.
///
/// See [`initialize()`].
///
/// *WSTP C API Documentation:* [WSENV](https://reference.wolfram.com/language/ref/c/WSENV.html).
pub struct WstpEnv {
    raw_env: sys::WSENV,
}

/// A WSTP link object.
///
/// [`WSClose()`][sys::WSClose] is called on the underlying `WSLINK` when
/// [`Drop::drop()`][WstpLink::drop] is called for a value of this type.
///
/// *WSTP C API Documentation:* [WSLINK](https://reference.wolfram.com/language/ref/c/WSLINK.html)
#[derive(Debug)]
pub struct WstpLink {
    raw_link: WSLINK,
}

//======================================
// Impls
//======================================

pub fn initialize() -> Result<WstpEnv, Error> {
    let raw_env: sys::WSENV;

    // TODO: Is this thread-safe?
    //       Is it safe to call WSInitialize() multiple times in the same process?
    unsafe {
        raw_env = sys::WSInitialize(std::ptr::null_mut());
    }

    if raw_env.is_null() {
        return Err(Error::custom(
            // TODO: Is there an internal error string which could be included here?
            format!("WSInitialize() failed"),
        ));
    }

    Ok(WstpEnv { raw_env })
}

impl WstpEnv {
    pub fn raw_env(&self) -> sys::WSENV {
        let WstpEnv { raw_env } = *self;

        raw_env
    }
}

impl WstpLink {
    /// Create a new Loopback type link.
    ///
    /// *WSTP C API Documentation:* [WSLoopbackOpen()](https://reference.wolfram.com/language/ref/c/WSLoopbackOpen.html)
    pub fn new_loopback(env: &WstpEnv) -> Result<Self, Error> {
        unsafe {
            let mut err: std::os::raw::c_int = sys::MLEOK as i32;
            let raw_link = sys::WSLoopbackOpen(env.raw_env, &mut err);

            if raw_link.is_null() || err != (sys::MLEOK as i32) {
                return Err(Error::from_code(err));
            }

            Ok(WstpLink::unchecked_new(raw_link))
        }
    }

    pub unsafe fn unchecked_new(raw_link: WSLINK) -> Self {
        WstpLink { raw_link }
    }

    /// Get the name of this link.
    ///
    /// *WSTP C API Documentation:* [WSName()](https://reference.wolfram.com/language/ref/c/WSName.html)
    pub fn name(&self) -> String {
        let WstpLink { raw_link } = *self;

        unsafe {
            let name: *const i8 = self::sys::WSName(raw_link as *mut _);
            CStr::from_ptr(name).to_str().unwrap().to_owned()
        }
    }

    /// Check if there is data ready to be read from this link.
    ///
    /// *WSTP C API Documentation:* [WSReady()](https://reference.wolfram.com/language/ref/c/WSReady.html)
    pub fn is_ready(&self) -> bool {
        let WstpLink { raw_link } = *self;

        unsafe { WSReady(raw_link) != 0 }
    }

    /// Read an expression off of this link.
    pub fn get_expr(&mut self) -> Result<Expr, Error> {
        let WstpLink { raw_link } = *self;

        unsafe { get_expr(raw_link) }
    }

    /// Write an expression to this link.
    pub fn put_expr(&mut self, expr: &Expr) -> Result<(), Error> {
        let WstpLink { raw_link: link } = *self;

        unsafe {
            WSNewPacket(link);

            let res = put_expr(link, expr);

            link_try!(link, WSEndPacket(link));

            res
        }
    }

    /// Returns a string describing the last error to occur on this link.
    ///
    /// TODO: If the most recent operation was successful, does the error message get
    ///       cleared?
    ///
    /// *WSTP C API Documentation:* [WSErrorMessage()](https://reference.wolfram.com/language/ref/c/WSErrorMessage.html)
    pub fn error_message(&self) -> Option<String> {
        let WstpLink { raw_link } = *self;

        let error = unsafe { error_message(raw_link) };

        error.map(|Error { message, code: _ }| message)
    }

    pub unsafe fn raw_link(&self) -> WSLINK {
        let WstpLink { raw_link } = *self;
        raw_link
    }

    /// Close this end of the link.
    ///
    /// *WSTP C API Documentation:* [WSClose](https://reference.wolfram.com/language/ref/c/WSClose.html)
    pub fn close(self) {
        // Note: The link is closed when `self` is dropped.
    }
}

unsafe fn error_message(link: WSLINK) -> Option<Error> {
    let code: i32 = sys::WSError(link);
    let message: *const i8 = WSErrorMessage(link);

    if code == (sys::MLEOK as i32) || message.is_null() {
        return None;
    }

    let cstr = CStr::from_ptr(message);
    let string: String = cstr.to_str().unwrap().to_owned();

    WSReleaseErrorMessage(link, message);

    WSClearError(link);

    return Some(Error {
        code: Some(code),
        message: string,
    });
}

unsafe fn error_message_or_unknown(link: WSLINK) -> Error {
    error_message(link)
        .unwrap_or_else(|| Error::custom("unknown error occurred on WSLINK".into()))
}

//======================================
// Read from the link
//======================================

unsafe fn get_expr(link: WSLINK) -> Result<Expr, Error> {
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
                    return Err(Error::custom(format!(
                        "NaN value passed on WSLINK cannot be used to construct an Expr"
                    )))
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
                None => {
                    return Err(Error::custom(format!(
                        "Symbol name `{}` has no context",
                        string
                    )))
                },
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
        _ => return Err(Error::custom(format!("unknown WSLINK type: {}", type_))),
    };

    Ok(expr)
}

//======================================
// Write to the link
//======================================

unsafe fn put_expr(link: WSLINK, expr: &Expr) -> Result<(), Error> {
    match expr.kind() {
        ExprKind::Normal(Normal { head, contents }) => {
            link_try!(link, WSPutType(link, i32::from(wl_wstp_sys::WSTKFUNC)));
            let contents_len =
                i32::try_from(contents.len()).expect("usize overflows i32");
            link_try!(link, WSPutArgCount(link, contents_len));

            let _: () = put_expr(link, &*head)?;

            for elem in contents {
                let _: () = put_expr(link, elem)?;
            }
        },
        ExprKind::Symbol(symbol) => {
            let cstring = CString::new(symbol.to_string()).unwrap();

            let len =
                i32::try_from(cstring.to_bytes().len()).expect("usize overflows i32");

            link_try!(
                link,
                WSPutUTF8Symbol(link, cstring.to_bytes().as_ptr(), len)
            );
        },
        ExprKind::String(string) => {
            let cstring = CString::new(string.clone())
                .expect("Expr string can not be stored in CString");

            let len =
                i32::try_from(cstring.to_bytes().len()).expect("usize overflows i32");

            link_try!(
                link,
                WSPutUTF8String(link, cstring.to_bytes().as_ptr(), len)
            );
        },
        ExprKind::Number(Number::Integer(int)) => {
            link_try!(link, WSPutInteger64(link, *int));
        },
        ExprKind::Number(Number::Real(real)) => {
            link_try!(link, WSPutReal64(link, **real));
        },
    }

    Ok(())
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

//======================================
// Drop impls
//======================================

impl Drop for WstpLink {
    fn drop(&mut self) {
        let WstpLink { raw_link } = *self;

        unsafe {
            sys::WSClose(raw_link);
        }
    }
}
