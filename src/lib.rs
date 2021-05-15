mod error;

use std::convert::TryFrom;
use std::ffi::{CStr, CString};

use wl_expr::{Expr, ExprKind, Normal, Number, Symbol};
use wl_wstp_sys::{
    WSEndPacket, WSErrorMessage, WSGetArgCount, WSGetInteger64, WSGetReal64, WSGetSymbol,
    WSGetType, WSGetUTF8String, WSNewPacket, WSPutArgCount, WSPutInteger64, WSPutReal64,
    WSPutType, WSPutUTF8String, WSPutUTF8Symbol, WSReady, WSReleaseErrorMessage,
    WSReleaseString, WSReleaseSymbol, WSLINK,
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
/// *WSTP C API Documentation:* [`WSENV`](https://reference.wolfram.com/language/ref/c/WSENV.html).
pub struct WstpEnv {
    raw_env: sys::WSENV,
}

/// A WSTP link object.
///
/// [`WSClose()`][sys::WSClose] is called on the underlying `WSLINK` when
/// [`Drop::drop()`][WstpLink::drop] is called for a value of this type.
///
/// *WSTP C API Documentation:* [`WSLINK`](https://reference.wolfram.com/language/ref/c/WSLINK.html)
///
/// *Wolfram Language Documentation:* [`LinkObject`](https://reference.wolfram.com/language/ref/LinkObject.html)
#[derive(Debug)]
pub struct WstpLink {
    raw_link: WSLINK,
}

/// Reference to string data borrowed from a [`WstpLink`].
///
/// `LinkStr` is returned from [`WstpLink::get_string_ref()`] and [`WstpLink::get_symbol()`].
///
/// When [`LinkStr::drop()`] is called, `WSReleaseString()` is used to deallocate the
/// underlying string.
pub struct LinkStr<'link> {
    link: &'link WstpLink,
    // Note: See `LinkStr::to_str()` for discussion of the safety reasons we *don't* store
    //       a `&str` field (even though that would have the benefit of paying the UTF-8
    //       validation penalty only once).
    c_string: *const u8,
    byte_length: usize,
    is_symbol: bool,
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

/// # Creating WSTP link objects
impl WstpLink {
    /// Create a new Loopback type link.
    ///
    /// *WSTP C API Documentation:* [`WSLoopbackOpen()`](https://reference.wolfram.com/language/ref/c/WSLoopbackOpen.html)
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


    /// Close this end of the link.
    ///
    /// *WSTP C API Documentation:* [`WSClose()`](https://reference.wolfram.com/language/ref/c/WSClose.html)
    pub fn close(self) {
        // Note: The link is closed when `self` is dropped.
    }
}

/// # Link properties
impl WstpLink {
    /// Get the name of this link.
    ///
    /// *WSTP C API Documentation:* [`WSName()`](https://reference.wolfram.com/language/ref/c/WSName.html)
    pub fn name(&self) -> String {
        let WstpLink { raw_link } = *self;

        unsafe {
            let name: *const i8 = self::sys::WSName(raw_link as *mut _);
            CStr::from_ptr(name).to_str().unwrap().to_owned()
        }
    }

    /// Check if there is data ready to be read from this link.
    ///
    /// *WSTP C API Documentation:* [`WSReady()`](https://reference.wolfram.com/language/ref/c/WSReady.html)
    pub fn is_ready(&self) -> bool {
        let WstpLink { raw_link } = *self;

        unsafe { WSReady(raw_link) != 0 }
    }

    /// Returns an [`Error`] describing the last error to occur on this link.
    ///
    /// # Examples
    ///
    /// **TODO:** Example of getting an error code.
    pub fn error(&self) -> Option<Error> {
        let WstpLink { raw_link } = *self;

        let (code, message): (i32, *const i8) =
            unsafe { (sys::WSError(raw_link), WSErrorMessage(raw_link)) };

        if code == (sys::MLEOK as i32) || message.is_null() {
            return None;
        }

        let string: String = unsafe {
            let cstr = CStr::from_ptr(message);
            let string = cstr.to_str().unwrap().to_owned();

            WSReleaseErrorMessage(raw_link, message);
            // TODO: Should this method clear the error? If it does, it should at least be
            //       '&mut self'.
            // WSClearError(link);

            string
        };

        return Some(Error {
            code: Some(code),
            message: string,
        });
    }

    /// Returns a string describing the last error to occur on this link.
    ///
    /// TODO: If the most recent operation was successful, does the error message get
    ///       cleared?
    ///
    /// *WSTP C API Documentation:* [`WSErrorMessage()`](https://reference.wolfram.com/language/ref/c/WSErrorMessage.html)
    pub fn error_message(&self) -> Option<String> {
        self.error().map(|Error { message, code: _ }| message)
    }

    /// Helper to create an [`Error`] instance even if the underlying link does not have
    /// an error code set.
    pub(crate) fn error_or_unknown(&self) -> Error {
        self.error()
            .unwrap_or_else(|| Error::custom("unknown error occurred on WSLINK".into()))
    }

    /// *WSTP C API Documentation:* [`WSLINK`](https://reference.wolfram.com/language/ref/c/WSLINK.html)
    pub unsafe fn raw_link(&self) -> WSLINK {
        let WstpLink { raw_link } = *self;
        raw_link
    }
}

/// # Reading and writing expressions
impl WstpLink {
    /// Read an expression off of this link.
    pub fn get_expr(&mut self) -> Result<Expr, Error> {
        unsafe { get_expr(self) }
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

    /// *WSTP C API Documentation:* [`WSGetInteger64()`](https://reference.wolfram.com/language/ref/c/WSGetInteger64.html)
    pub fn get_i64(&mut self) -> Result<i64, Error> {
        let mut int = 0;
        if unsafe { WSGetInteger64(self.raw_link, &mut int) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(int)
    }

    /// *WSTP C API Documentation:* [`WSGetReal64()`](https://reference.wolfram.com/language/ref/c/WSGetReal64.html)
    pub fn get_f64(&mut self) -> Result<f64, Error> {
        let mut real: f64 = 0.0;
        if unsafe { WSGetReal64(self.raw_link, &mut real) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(real)
    }

    // TODO:
    //     Reserving the name `get_str()` in case it's possible in the future to implement
    //     implement a `WstpLink::get_str() -> &str` method. It may be safe to do that if
    //     we either:
    //
    //       * Keep track of all the strings we need to call `WSReleaseString` on, and
    //         then do so in `WstpLink::drop()`.
    //       * Verify that we don't need to explicitly deallocate the string data, because
    //         they will be deallocated when the mempool is freed (presumably during
    //         WSClose()?).

    /// *WSTP C API Documentation:* [`WSGetUTF8String()`](https://reference.wolfram.com/language/ref/c/WSGetUTF8String.html)
    pub fn get_string_ref<'link>(&'link mut self) -> Result<LinkStr<'link>, Error> {
        let mut c_string: *const u8 = std::ptr::null();
        let mut num_bytes: i32 = 0;
        let mut num_chars = 0;

        if unsafe {
            WSGetUTF8String(self.raw_link, &mut c_string, &mut num_bytes, &mut num_chars)
        } == 0
        {
            // NOTE: According to the documentation, we do NOT have to release
            //      `string` if the function returns an error.
            return Err(self.error_or_unknown());
        }

        let num_bytes = usize::try_from(num_bytes).unwrap();

        Ok(LinkStr {
            link: self,
            c_string,
            byte_length: num_bytes,
            // Needed to control whether `WSReleaseString` or `WSReleaseSymbol` is called.
            is_symbol: false,
        })
    }

    /// Convenience wrapper around [`WstpLink::get_string_ref()`].
    pub fn get_string(&mut self) -> Result<String, Error> {
        Ok(self.get_string_ref()?.to_str().to_owned())
    }
}

impl<'link> LinkStr<'link> {
    /// Get the UTF-8 string data.
    ///
    /// # Panics
    ///
    /// This function will panic if the contents of the string are not valid UTF-8.
    pub fn to_str<'s>(&'s self) -> &'s str {
        let LinkStr {
            link: _,
            c_string,
            byte_length,
            is_symbol: _,
        } = *self;

        // Safety: Assert this pre-condition of `slice::from_raw_parts()`.
        assert!(byte_length < usize::try_from(isize::MAX).unwrap());

        // SAFETY:
        //     It is important that the lifetime of `bytes` is tied to `self` and NOT to
        //     'link. A `&'link str` could outlive the `LinkStr` object, which would lead
        //     to a a use-after-free bug because the string data is deallocated when
        //     `LinkStr` is dropped.
        let bytes: &'s [u8] =
            unsafe { std::slice::from_raw_parts(c_string, byte_length) };

        // TODO: Optimization: Do we trust WSTP enough to always produce valid UTF-8 to
        //       use `str::from_utf8_unchecked()` here? If a client writes malformed data
        //       with WSPutUTF8String, does WSTP validate it and return an error, or would
        //       it be passed through to unsuspecting us?
        // This function will panic if `c_string` is not valid UTF-8.
        std::str::from_utf8(bytes).expect("WSTP returned non-UTF-8 string")
    }
}

impl<'link> Drop for LinkStr<'link> {
    fn drop(&mut self) {
        let LinkStr {
            link,
            c_string,
            byte_length: _,
            is_symbol,
        } = *self;

        let c_string = c_string as *const i8;

        // Deallocate the string data.
        match is_symbol {
            true => unsafe { WSReleaseSymbol(link.raw_link, c_string) },
            false => unsafe { WSReleaseString(link.raw_link, c_string) },
        }
    }
}

unsafe fn error_message_or_unknown(link: WSLINK) -> Error {
    WstpLink::unchecked_new(link).error_or_unknown()
}

//======================================
// Read from the link
//======================================

unsafe fn get_expr(safe_link: &mut WstpLink) -> Result<Expr, Error> {
    use wl_wstp_sys::{WSTKERR, WSTKFUNC, WSTKINT, WSTKREAL, WSTKSTR, WSTKSYM};

    let link = safe_link.raw_link;

    let type_: i32 = WSGetType(link);

    if type_ == WSTKERR as i32 {
        return Err(error_message_or_unknown(link));
    }

    let expr: Expr = match type_ as u8 {
        WSTKINT => Expr::number(Number::Integer(safe_link.get_i64()?)),
        WSTKREAL => {
            let real: wl_expr::F64 = match wl_expr::F64::new(safe_link.get_f64()?) {
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
        WSTKSTR => Expr::string(safe_link.get_string_ref()?.to_str()),
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

            let head = safe_link.get_expr()?;

            let mut contents = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                contents.push(safe_link.get_expr()?);
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
