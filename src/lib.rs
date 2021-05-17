//! Bindings to [Wolfram Symbolic Transfer Protocol (WSTP)](https://www.wolfram.com/wstp/).
//!
//! This crate provides a set of safe and ergonomic bindings to the WSTP library, used to
//! transfer Wolfram Language expressions between programs.

mod error;

use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt::{self, Display};

use wl_expr::{Expr, ExprKind, Normal, Number, Symbol};
use wl_wstp_sys::{
    WSErrorMessage, WSGetArgCount, WSGetInteger64, WSGetReal64, WSGetType,
    WSGetUTF8String, WSPutArgCount, WSPutInteger64, WSPutReal64, WSPutType,
    WSPutUTF8String, WSPutUTF8Symbol, WSReady, WSReleaseErrorMessage, WSReleaseString,
    WSReleaseSymbol, WSLINK,
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

/// # Safety
///
/// `WstpLink` links can be sent between threads, but they cannot be used from multiple
/// threads at once (unless `WSEnableLinkLock()` has been called on the link). So `WstpLink`
/// satisfies [`Send`] but not [`Sync`].
///
/// **TODO:**
///   Add a wrapper type for `WstpLink` which enforces that `WSEnableLinkLock()`
///   has been called, and implements [`Sync`].
unsafe impl Send for WstpLink {}

/// Reference to string data borrowed from a [`WstpLink`].
///
/// `LinkStr` is returned from [`WstpLink::get_string_ref()`] and [`WstpLink::get_symbol_ref()`].
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

/// Transport protocol used to communicate between two [`WstpLink`] end points.
pub enum Protocol {
    /// Protocol type optimized for communication between two [`WstpLink`] end points
    /// from within the same OS process.
    IntraProcess,
    /// Protocol type optimized for communication between two [`WstpLink`] end points
    /// from the same machine — but not necessarily in the same OS process — using [shared
    /// memory](https://en.wikipedia.org/wiki/Shared_memory).
    SharedMemory,
    /// Protocol type for communication between two [`WstpLink`] end points reachable
    /// across a network connection.
    TCPIP,
}

//======================================
// Impls
//======================================

/// *WSTP C API Documentation:* [`WSInitialize()`](https://reference.wolfram.com/language/ref/c/WSInitialize.html)
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

    /// Create a new named WSTP link using `protocol`.
    pub fn listen(env: &WstpEnv, protocol: Protocol, name: &str) -> Result<Self, Error> {
        let protocol_string = protocol.to_string();

        let strings: &[&str] = &[
            "-wstp",
            "-linkmode",
            "listen",
            "-linkprotocol",
            protocol_string.as_str(),
            "-linkname",
            name,
            // Prevent "Link created on: .." message from being printed.
            "-linkoptions",
            "MLDontInteract",
        ];

        WstpLink::open_with_args(env, strings)
    }

    /// Connect to an existing named WSTP link.
    pub fn connect(env: &WstpEnv, protocol: Protocol, name: &str) -> Result<Self, Error> {
        let protocol_string = protocol.to_string();

        let strings: &[&str] = &[
            "-wstp",
            // "-linkconnect",
            "-linkmode",
            "connect",
            "-linkprotocol",
            protocol_string.as_str(),
            "-linkname",
            name,
        ];

        WstpLink::open_with_args(env, strings)
    }

    /// *WSTP C API Documentation:* [`WSOpenArgcArgv()`](https://reference.wolfram.com/language/ref/c/WSOpenArgcArgv.html)
    ///
    /// This function can be used to create a `WstpLink` of any protocol and mode. Prefer
    /// to use one of constructor methods listed below when you know the type of link to
    /// be created.
    ///
    /// * [`WstpLink::listen()`]
    /// * [`WstpLink::connect()`]
    /// * [`WstpLink::launch()`]
    /// * [`WstpLink::parent_connect()`]
    pub fn open_with_args(env: &WstpEnv, args: &[&str]) -> Result<Self, Error> {
        // NOTE: Before returning, we must convert these back into CString's to
        //       deallocate them.
        let mut c_strings: Vec<*mut i8> = args
            .into_iter()
            .map(|&str| {
                CString::new(str)
                    .expect("failed to create CString from WSTP link open argument")
                    .into_raw()
            })
            .collect();

        let mut err: std::os::raw::c_int = sys::MLEOK as i32;

        let raw_link = unsafe {
            sys::WSOpenArgcArgv(
                env.raw_env(),
                i32::try_from(c_strings.len()).unwrap(),
                c_strings.as_mut_ptr(),
                &mut err,
            )
        };

        // Convert the `*mut i8` C strings back into owned CString's, so that they are
        // deallocated.
        for c_string in c_strings {
            unsafe {
                let _ = CString::from_raw(c_string);
            }
        }

        if raw_link.is_null() || err != (sys::MLEOK as i32) {
            return Err(Error::from_code(err));
        }

        Ok(WstpLink { raw_link })
    }

    pub unsafe fn unchecked_new(raw_link: WSLINK) -> Self {
        WstpLink { raw_link }
    }

    /// *WSTP C API Documentation:* [`WSActivate()`](https://reference.wolfram.com/language/ref/c/WSActivate.html)
    pub fn activate(&mut self) -> Result<(), Error> {
        // Note: WSActivate() returns 0 in the event of an error, and sets an error
        //       code retrievable by WSError().
        if unsafe { sys::WSActivate(self.raw_link) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
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
    /// *WSTP C API Documentation:* [`WSLinkName()`](https://reference.wolfram.com/language/ref/c/WSLinkName.html)
    pub fn link_name(&self) -> String {
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
    /// Flush out any buffers containing data waiting to be sent on this link.
    ///
    /// *WSTP C API Documentation:* [`WSFlush()`](https://reference.wolfram.com/language/ref/c/WSFlush.html)
    pub fn flush(&mut self) -> Result<(), Error> {
        if unsafe { sys::WSFlush(self.raw_link) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Read an expression off of this link.
    pub fn get_expr(&mut self) -> Result<Expr, Error> {
        get_expr(self)
    }

    /// Write an expression to this link.
    pub fn put_expr(&mut self, expr: &Expr) -> Result<(), Error> {
        match expr.kind() {
            ExprKind::Normal(Normal { head, contents }) => {
                let contents_len =
                    i32::try_from(contents.len()).expect("usize overflows i32");

                unsafe {
                    if WSPutType(self.raw_link, i32::from(wl_wstp_sys::WSTKFUNC)) == 0 {
                        return Err(self.error_or_unknown());
                    }
                    if WSPutArgCount(self.raw_link, contents_len) == 0 {
                        return Err(self.error_or_unknown());
                    }
                }

                let _: () = self.put_expr(&*head)?;

                for elem in contents {
                    let _: () = self.put_expr(elem)?;
                }
            },
            ExprKind::Symbol(symbol) => {
                self.put_symbol(symbol.as_str())?;
            },
            ExprKind::String(string) => {
                self.put_str(string.as_str())?;
            },
            ExprKind::Number(Number::Integer(int)) => {
                self.put_i64(*int)?;
            },
            ExprKind::Number(Number::Real(real)) => {
                self.put_f64(**real)?;
            },
        }

        Ok(())
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

    /// *WSTP C API Documentation:* [`WSGetUTF8Symbol()`](https://reference.wolfram.com/language/ref/c/WSGetUTF8Symbol.html)
    pub fn get_symbol_ref<'link>(&'link mut self) -> Result<LinkStr<'link>, Error> {
        let mut c_string: *const u8 = std::ptr::null();
        let mut num_bytes: i32 = 0;
        let mut num_chars = 0;

        if unsafe {
            sys::WSGetUTF8Symbol(
                self.raw_link,
                &mut c_string,
                &mut num_bytes,
                &mut num_chars,
            )
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
            is_symbol: true,
        })
    }

    /// *WSTP C API Documentation:* [`WSPutInteger64()`](https://reference.wolfram.com/language/ref/c/WSPutInteger64.html)
    pub fn put_i64(&mut self, value: i64) -> Result<(), Error> {
        if unsafe { WSPutInteger64(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutReal64()`](https://reference.wolfram.com/language/ref/c/WSPutReal64.html)
    pub fn put_f64(&mut self, value: f64) -> Result<(), Error> {
        if unsafe { WSPutReal64(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutUTF8String()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8String.html)
    pub fn put_str(&mut self, string: &str) -> Result<(), Error> {
        // TODO: Optimization:
        //     This intermediate CString allocation may not actually be necessary. Because
        //     WSPutUTF8String() takes a pointer + length pair, it's possible it doesn't
        //     require that the string be NULL terminated. I'm not confident that is the
        //     case though, and it isn't explicitly documented one way or the other.
        //     Investigate this in the WSTP sources, and fix this if possible. If fixed,
        //     be sure to include this assertion (`str`'s can contain NULL bytes, and
        //     I have much less confidence that older parts of WSTP are strict about not
        //     using strlen() on strings internally).
        //
        //         assert!(!string.bytes().any(|byte| byte == 0));
        let c_string = CString::new(string).unwrap();

        let len = i32::try_from(c_string.as_bytes().len()).expect("usize overflows i32");
        let ptr = c_string.as_ptr() as *const u8;

        if unsafe { WSPutUTF8String(self.raw_link, ptr, len) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutUTF8Symbol()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8Symbol.html)
    pub fn put_symbol(&mut self, symbol: &str) -> Result<(), Error> {
        let c_string = CString::new(symbol).unwrap();

        let len = i32::try_from(c_string.as_bytes().len()).expect("usize overflows i32");
        let ptr = c_string.as_ptr() as *const u8;

        if unsafe { WSPutUTF8Symbol(self.raw_link, ptr, len) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
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

//======================================
// Read from the link
//======================================

fn get_expr(link: &mut WstpLink) -> Result<Expr, Error> {
    use wl_wstp_sys::{WSTKERR, WSTKFUNC, WSTKINT, WSTKREAL, WSTKSTR, WSTKSYM};

    let type_: i32 = unsafe { WSGetType(link.raw_link) };

    if type_ == WSTKERR as i32 {
        return Err(link.error_or_unknown());
    }

    let expr: Expr = match type_ as u8 {
        WSTKINT => Expr::number(Number::Integer(link.get_i64()?)),
        WSTKREAL => {
            let real: wl_expr::F64 = match wl_expr::F64::new(link.get_f64()?) {
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
        WSTKSTR => Expr::string(link.get_string_ref()?.to_str()),
        WSTKSYM => {
            let symbol_link_str = link.get_symbol_ref()?;
            let symbol_str = symbol_link_str.to_str();

            let symbol: Symbol = match wl_parse::parse_symbol(symbol_str) {
                Some(sym) => sym,
                None => {
                    return Err(Error::custom(format!(
                        "Symbol name `{}` has no context",
                        symbol_str
                    )))
                },
            };

            Expr::symbol(symbol)
        },
        WSTKFUNC => {
            let mut arg_count = 0;

            if unsafe { WSGetArgCount(link.raw_link, &mut arg_count) } == 0 {
                return Err(link.error_or_unknown());
            }

            let arg_count = usize::try_from(arg_count)
                // This really shouldn't happen on any modern 32/64 bit OS. If this
                // condition *is* reached, it's more likely going to be do to an ABI or
                // numeric environment handling issue.
                .expect("WSTKFUNC argument count could not be converted to usize");

            let head = link.get_expr()?;

            let mut contents = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                contents.push(link.get_expr()?);
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

//======================================
// Utilities
//======================================

//======================================
// Formatting impls
//======================================

impl Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = match self {
            Protocol::IntraProcess => "IntraProcess",
            Protocol::SharedMemory => "SharedMemory",
            Protocol::TCPIP => "TCPIP",
        };

        write!(f, "{}", str)
    }
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
