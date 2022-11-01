//! Bindings to the [Wolfram Symbolic Transfer Protocol (WSTP)](https://www.wolfram.com/wstp/).
//!
//! This crate provides a set of safe and ergonomic bindings to the WSTP library, used to
//! transfer Wolfram Language expressions between programs.
//!
//! # Quick Examples
//!
//! ### Loopback links
//!
//! Write an expression to a loopback [`Link`], and then read it back from the same link
//! object:
//!
//! ```
//! use wstp::Link;
//!
//! # fn example() -> Result<(), wstp::Error> {
//! let mut link = Link::new_loopback()?;
//!
//! // Write the expression {"a", "b", "c"}
//! link.put_function("System`List", 3)?;
//! link.put_str("a")?;
//! link.put_str("b")?;
//! link.put_str("c")?;
//!
//! // Read back the expression, concatenating the elements as we go:
//! let mut buffer = String::new();
//!
//! for _ in 0 .. link.test_head("System`List")? {
//!     buffer.push_str(link.get_string_ref()?.as_str())
//! }
//!
//! assert_eq!(buffer, "abc");
//! # Ok(())
//! # }
//! #
//! # example();
//! ```
//!
//! ### Full-duplex links
//!
//! Transfer the expression `"hello!"` from one [`Link`] endpoint to another:
//!
//! ```
//! use std::{thread, time::Duration};
//! use wstp::{Link, Protocol};
//!
//! // Start a background thread with a listen()'ing link.
//! let listening_thread = thread::spawn(|| {
//!     // This will block until an incoming connection is made.
//!     let mut link = Link::listen(Protocol::SharedMemory, "my-link").unwrap();
//!
//!     link.put_str("hello!").unwrap();
//! });
//!
//! // Give the listening thread time to start before we
//! // try to connect to it.
//! thread::sleep(Duration::from_millis(20));
//!
//! let mut link = Link::connect(Protocol::SharedMemory, "my-link").unwrap();
//! assert_eq!(link.get_string().unwrap(), "hello!");
//! ```
//!
//! # What is WSTP?
//!
//! The name Wolfram Symbolic Transfer Protocol (WSTP) refers to two interrelated things:
//!
//! * The WSTP *protocol*
//! * The WSTP *library*, which provides the canonical implementation of the protocol via
//!   a C API.
//!
//! ### The protocol
//!
//! At a high level, the WSTP defines a full-duplex communication channel optimized for
//! the transfer of Wolfram Language expressions between two endpoints. A WSTP
//! connection typically has exactly two [`Link`] endpoints
//! ([loopback links][Link::new_loopback] are the only exception). A connection between two
//! endpoints is established when one endpoint is created using [`Link::listen()`], and
//! another endpoint is created using [`Link::connect()`].
//!
//! At a lower level, WSTP is actually three protocols:
//!
//! * [`IntraProcess`][Protocol::IntraProcess]
//! * [`SharedMemory`][Protocol::SharedMemory]
//! * [`TCPIP`][Protocol::TCPIP]
//!
//! which are represented by the [`Protocol`] enum. Each lower-level protocol is optimized
//! for usage within a particular domain. For example, `IntraProcess` is the best link
//! type to use when both [`Link`] endpoints reside within the same OS process, and
//! `TCPIP` links can be used when the [`Link`] endpoints reside on different
//! computers that are reachable across the network.
//!
//! Given that the different [`Protocol`] types use different mechanisms to transfer data,
//! it is not possible to create a connection between links of different types. E.g. a
//! `TCPIP` type link cannot connect to a `SharedMemory` link, even if both endpoints were
//! created on the same computer and in the same process.
//!
// TODO: The packet protocol.
//!
//! ### The library
//!
//! The WSTP library is distributed as part the Wolfram Language as both a static and
//! dynamic library. The WSTP SDK is present in the file system layout of the Mathematica,
//! Wolfram Desktop, and [Wolfram Engine][WolframEngine] applications. The `wstp` crate
//! is built on top of the [WSTP C API][CFunctions].
//!
//! When using the `wstp` crate as a dependency, the `wstp` crate's cargo build script
//! will use [`wolfram-app-discovery`][wolfram-app-discovery] to automatically find any
//! local installations of the Wolfram Language, and will link against the WSTP static
//! library located within.
//!
//! The [Wolfram Engine][WolframEngine] can be downloaded and used for free for
//! non-commercial or pre-production uses. A license must be purchased when used as part
//! of a commercial or production-level product. See the *Licensing and Terms of
//! Use* section in the [Wolfram Engine FAQ][WE-FAQ] for details.
//!
// TODO: Mention package manager downloads of WolframEngine.
//!
//!
//! # Related Links
//!
//! * [WSTP and External Program Communication](https://reference.wolfram.com/language/tutorial/WSTPAndExternalProgramCommunicationOverview.html)
//! * [How WSTP Is Used](https://reference.wolfram.com/language/tutorial/HowWSTPIsUsed.html)
//! * [Alphabetical Listing of WSTP C Functions][CFunctions]
//!
//! ### Licensing
//!
//! Usage of the WSTP library is subject to the terms of the
//! [MathLink License Agreement](https://www.wolfram.com/legal/agreements/mathlink.html).
//!
//!
//! [WolframEngine]: https://www.wolfram.com/engine/
//! [WE-FAQ]: https://www.wolfram.com/engine/faq/
//! [CFunctions]: https://reference.wolfram.com/language/guide/AlphabeticalListingOfWSTPCFunctions.html
//!
//! [wolfram-app-discovery]: https://crates.io/crates/wolfram-app-discovery

#![warn(missing_docs)]


mod env;
mod error;
mod link_server;
mod wait;

mod get;
mod put;

mod strx;

pub mod kernel;

/// Ensure that doc tests in the README.md file get run.
#[doc(hidden)]
mod test_readme {
    #![doc = include_str!("../README.md")]
}


use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt::{self, Display};
use std::net;

use wolfram_expr::{Expr, ExprKind, Number, Symbol};
use wstp_sys::{WSErrorMessage, WSReady, WSReleaseErrorMessage, WSLINK};

//-----------------------------------
// Public re-exports and type aliases
//-----------------------------------

/// Raw bindings to the [WSTP C API][CFunctions].
///
/// [CFunctions]: https://reference.wolfram.com/language/guide/AlphabeticalListingOfWSTPCFunctions.html
#[doc(inline)]
pub use wstp_sys as sys;

pub use crate::{
    env::shutdown,
    error::Error,
    get::{Array, LinkStr, Token, TokenType},
    link_server::LinkServer,
    strx::{Ucs2Str, Utf16Str, Utf32Str, Utf8Str},
};

// TODO: Make this function public from `wstp`?
pub(crate) use env::stdenv;


//======================================
// Source
//======================================

/// WSTP link endpoint.
///
/// [`WSClose()`][sys::WSClose] is called on the underlying [`WSLINK`] when
/// [`Drop::drop()`][Link::drop] is called for a value of this type.
///
/// *WSTP C API Documentation:* [`WSLINK`](https://reference.wolfram.com/language/ref/c/WSLINK.html)
///
/// *Wolfram Language Documentation:* [`LinkObject`](https://reference.wolfram.com/language/ref/LinkObject.html)
#[derive(Debug)]
#[derive(ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct Link {
    raw_link: WSLINK,
}

impl Link {
    /// Transmute a `&mut WSLINK` into a `&mut Link`.
    ///
    /// This operation enables usage of the safe [`Link`] wrapper type without assuming
    /// ownership over the underying raw `WSLINK`.
    ///
    /// Use this function to construct a [`Link`] from a borrowed
    /// [`WSLINK`][crate::sys::WSLINK]. This function should be used in LibraryLink
    /// functions loaded via [`LibraryFunctionLoad`][LibraryFunctionLoad] instead of
    /// [`Link::unchecked_new()`].
    ///
    /// [LibraryFunctionLoad]: https://reference.wolfram.com/language/ref/LibraryFunctionLoad.html
    ///
    /// # Safety
    ///
    /// For this operation to be safe, the caller must ensure:
    ///
    /// * the `WSLINK` is validly initialized.
    /// * they have unique ownership of the `WSLINK` value; no aliasing is possible.
    ///
    /// and the maintainer of this functionality must ensure:
    ///
    /// * The [`Link`] type is a `#[repr(transparent)]` wrapper around around a
    ///   single field of type [`WSLINK`][crate::sys::WSLINK].
    #[inline]
    #[ref_cast::ref_cast_custom]
    pub unsafe fn unchecked_ref_cast_mut(from: &mut WSLINK) -> &mut Self;
}

/// # Safety
///
/// [`Link`]s can be sent between threads, but they cannot be used from multiple
/// threads at once (unless `WSEnableLinkLock()` has been called on the link). So [`Link`]
/// satisfies [`Send`] but not [`Sync`].
///
/// **TODO:**
///   Add a wrapper type for [`Link`] which enforces that `WSEnableLinkLock()`
///   has been called, and implements [`Sync`].
unsafe impl Send for Link {}

/// Transport protocol used to communicate between two [`Link`] end points.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// Protocol type optimized for communication between two [`Link`] end points
    /// from within the same OS process.
    IntraProcess,
    /// Protocol type optimized for communication between two [`Link`] end points
    /// from the same machine — but not necessarily in the same OS process — using [shared
    /// memory](https://en.wikipedia.org/wiki/Shared_memory).
    SharedMemory,
    /// Protocol type for communication between two [`Link`] end points reachable
    /// across a network connection.
    TCPIP,
}

//======================================
// Impls
//======================================

/// # Creating WSTP link objects
impl Link {
    /// Create a new Loopback type link.
    ///
    /// *WSTP C API Documentation:* [`WSLoopbackOpen()`](https://reference.wolfram.com/language/ref/c/WSLoopbackOpen.html)
    pub fn new_loopback() -> Result<Self, Error> {
        unsafe {
            let mut err: std::os::raw::c_int = sys::MLEOK;
            let raw_link = sys::WSLoopbackOpen(stdenv()?.raw_env, &mut err);

            if raw_link.is_null() || err != sys::MLEOK {
                return Err(Error::from_code(err));
            }

            Ok(Link::unchecked_new(raw_link))
        }
    }

    /// Create a new named WSTP link using `protocol`.
    pub fn listen(protocol: Protocol, name: &str) -> Result<Self, Error> {
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

        Link::open_with_args(strings)
    }

    /// Connect to an existing named WSTP link.
    pub fn connect(protocol: Protocol, name: &str) -> Result<Self, Error> {
        Link::connect_with_options(protocol, name, &[])
    }

    /// Create a new WSTP [`TCPIP`][Protocol::TCPIP] link bound to `addr`.
    ///
    /// If `addr` yields multiple addresses, listening will be attempted with each of the
    /// addresses until one succeeds and returns the listener. If none of the addresses
    /// succeed in creating a listener, the error returned from the last attempt
    /// (the last address) is returned.
    pub fn tcpip_listen<A: net::ToSocketAddrs>(addr: A) -> Result<Self, Error> {
        let addrs = addr.to_socket_addrs().map_err(|err| {
            Error::custom(format!("error connecting to TCPIP Link address: {}", err))
        })?;

        // Try each address, returning the first one which binds for listening successfully.
        for_each_addr(addrs.collect(), |addr| {
            Link::listen(Protocol::TCPIP, &tcpip_link_name(&addr))
        })
    }

    /// Connect to an existing WSTP [`TCPIP`][Protocol::TCPIP] link listening at `addr`.
    ///
    /// If `addr` yields multiple addresses, a connection will be attempted with each of
    /// the addresses until a connection is successful. If none of the addresses result
    /// in a successful connection, the error returned from the last connection attempt
    /// (the last address) is returned.
    pub fn tcpip_connect<A: net::ToSocketAddrs>(addr: A) -> Result<Self, Error> {
        let addrs = addr.to_socket_addrs().map_err(|err| {
            Error::custom(format!("error connecting to TCPIP Link address: {}", err))
        })?;

        // Try each address, returning the first one which connects successfully.
        for_each_addr(addrs.collect(), |addr| {
            Link::connect(Protocol::TCPIP, &tcpip_link_name(&addr))
        })
    }

    /// Open a WSTP [`Protocol::TCPIP`] connection to a [`LinkServer`].
    ///
    /// If `addrs` yields multiple addresses, a connection will be attempted with each of
    /// the addresses until a connection is successful. If none of the addresses result
    /// in a successful connection, the error returned from the last connection attempt
    /// (the last address) is returned.
    pub fn connect_to_link_server<A: net::ToSocketAddrs>(
        addrs: A,
    ) -> Result<Self, Error> {
        let addrs = addrs.to_socket_addrs().map_err(|err| {
            Error::custom(format!("error connecting to LinkServer address: {}", err))
        })?;

        // Try each address, returning the first one which connects successfully.
        for_each_addr(addrs.collect(), |addr| {
            let mut link = Link::connect_with_options(
                Protocol::TCPIP,
                &tcpip_link_name(&addr),
                // Pass the magic option which signals that we're connecting to a
                // LinkServer, not just a normal Link.
                &["MLUseUUIDTCPIPConnection"],
            )?;

            // TODO: Should we activate here, or let the caller do this?
            let () = link.activate()?;

            return Ok(link);
        })
    }

    #[allow(missing_docs)]
    pub fn connect_with_options(
        protocol: Protocol,
        name: &str,
        options: &[&str],
    ) -> Result<Self, Error> {
        let protocol_string = protocol.to_string();

        let mut strings: Vec<&str> = vec![
            "-wstp",
            // "-linkconnect",
            "-linkmode",
            "connect",
            "-linkprotocol",
            protocol_string.as_str(),
            "-linkname",
            name,
        ];

        if !options.is_empty() {
            strings.push("-linkoptions");
            strings.extend(options);
        }

        Link::open_with_args(&strings)
    }

    /// *WSTP C API Documentation:* [`WSOpenArgcArgv()`](https://reference.wolfram.com/language/ref/c/WSOpenArgcArgv.html)
    ///
    /// This function can be used to create a [`Link`] of any protocol and mode. Prefer
    /// to use one of the constructor methods listed below when you know the type of link
    /// to be created.
    ///
    /// * [`Link::listen()`]
    /// * [`Link::connect()`]
    /// * [`Link::tcpip_listen()`]
    /// * [`Link::tcpip_connect()`]
    /// * [`Link::connect_to_link_server()`]
    // * [`Link::launch()`]
    // * [`Link::parent_connect()`]
    pub fn open_with_args(args: &[&str]) -> Result<Self, Error> {
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

        let mut err: std::os::raw::c_int = sys::MLEOK;

        let raw_link = unsafe {
            sys::WSOpenArgcArgv(
                stdenv()?.raw_env,
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

        if raw_link.is_null() || err != sys::MLEOK {
            return Err(Error::from_code(err));
        }

        Ok(Link { raw_link })
    }

    /// Construct a [`Link`] from a raw [`WSLINK`] pointer.
    pub unsafe fn unchecked_new(raw_link: WSLINK) -> Self {
        Link { raw_link }
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
impl Link {
    /// Get the name of this link.
    ///
    /// *WSTP C API Documentation:* [`WSLinkName()`](https://reference.wolfram.com/language/ref/c/WSLinkName.html)
    pub fn link_name(&self) -> String {
        let Link { raw_link } = *self;

        unsafe {
            let name: *const i8 = self::sys::WSName(raw_link as *mut _);
            CStr::from_ptr(name).to_str().unwrap().to_owned()
        }
    }

    /// Check if there is data ready to be read from this link.
    ///
    /// *WSTP C API Documentation:* [`WSReady()`](https://reference.wolfram.com/language/ref/c/WSReady.html)
    pub fn is_ready(&self) -> bool {
        let Link { raw_link } = *self;

        unsafe { WSReady(raw_link) != 0 }
    }

    /// *WSTP C API Documentation:* [`WSIsLinkLoopback()`](https://reference.wolfram.com/language/ref/c/WSIsLinkLoopback.html)
    pub fn is_loopback(&self) -> bool {
        let Link { raw_link } = *self;

        1 == unsafe { sys::WSIsLinkLoopback(raw_link) }
    }

    /// Returns an [`Error`] describing the last error to occur on this link.
    ///
    /// # Examples
    ///
    /// **TODO:** Example of getting an error code.
    pub fn error(&self) -> Option<Error> {
        let Link { raw_link } = *self;

        let (code, message): (i32, *const i8) =
            unsafe { (sys::WSError(raw_link), WSErrorMessage(raw_link)) };

        if code == sys::MLEOK || message.is_null() {
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

    /// Clear errors on this link.
    ///
    /// *WSTP C API Documentation:* [`WSClearError()`](https://reference.wolfram.com/language/ref/c/WSClearError.html)
    pub fn clear_error(&mut self) {
        let Link { raw_link } = *self;

        unsafe {
            sys::WSClearError(raw_link);
        }
    }

    /// *WSTP C API Documentation:* [`WSLINK`](https://reference.wolfram.com/language/ref/c/WSLINK.html)
    pub unsafe fn raw_link(&self) -> WSLINK {
        let Link { raw_link } = *self;
        raw_link
    }

    /// *WSTP C API Documentation:* [`WSUserData`](https://reference.wolfram.com/language/ref/c/WSUserData.html)
    pub unsafe fn user_data(&self) -> (*mut std::ffi::c_void, sys::WSUserFunction) {
        let Link { raw_link } = *self;

        let mut user_func: sys::WSUserFunction = None;

        let data_obj: *mut std::ffi::c_void = sys::WSUserData(raw_link, &mut user_func);

        (data_obj, user_func)
    }

    /// *WSTP C API Documentation:* [`WSSetUserData`](https://reference.wolfram.com/language/ref/c/WSSetUserData.html)
    pub unsafe fn set_user_data(
        &mut self,
        data_obj: *mut std::ffi::c_void,
        user_func: sys::WSUserFunction,
    ) {
        let Link { raw_link } = *self;

        sys::WSSetUserData(raw_link, data_obj, user_func);
    }
}

/// # Reading and writing expressions
impl Link {
    /// Flush out any buffers containing data waiting to be sent on this link.
    ///
    /// *WSTP C API Documentation:* [`WSFlush()`](https://reference.wolfram.com/language/ref/c/WSFlush.html)
    pub fn flush(&mut self) -> Result<(), Error> {
        if unsafe { sys::WSFlush(self.raw_link) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSGetNext()`](https://reference.wolfram.com/language/ref/c/WSGetNext.html)
    pub fn raw_get_next(&mut self) -> Result<i32, Error> {
        let type_ = unsafe { sys::WSGetNext(self.raw_link) };

        if type_ == sys::WSTKERR {
            return Err(self.error_or_unknown());
        }

        Ok(type_)
    }

    /// *WSTP C API Documentation:* [`WSNextPacket()`](https://reference.wolfram.com/language/ref/c/WSNextPacket.html)
    pub fn raw_next_packet(&mut self) -> Result<i32, Error> {
        let type_ = unsafe { sys::WSNextPacket(self.raw_link) };

        if type_ == sys::ILLEGALPKT {
            return Err(self.error_or_unknown());
        }

        Ok(type_)
    }

    /// *WSTP C API Documentation:* [`WSNewPacket()`](https://reference.wolfram.com/language/ref/c/WSNewPacket.html)
    pub fn new_packet(&mut self) -> Result<(), Error> {
        if unsafe { sys::WSNewPacket(self.raw_link) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Read an expression off of this link.
    pub fn get_expr(&mut self) -> Result<Expr, Error> {
        self.get_expr_with_resolver(&mut |_| None)
    }

    // TODO: This needs a bit more design work before being made public. For starters,
    //       you have to pass a closure to it using `get_expr_with_resolver(&mut |_| ...)`
    //       which looks out of place. Using `dyn FnMut()` is to avoid having to
    //       monomorphize different copies of `get_expr_with_resolver()`
    #[doc(hidden)]
    pub fn get_expr_with_resolver(
        &mut self,
        mut resolver: &mut dyn FnMut(&str) -> Option<Symbol>,
    ) -> Result<Expr, Error> {
        let value = self.get_token()?;

        let expr: Expr = match value {
            Token::Integer(value) => Expr::from(value),
            Token::Real(value) => {
                let real: wolfram_expr::F64 = match wolfram_expr::F64::new(value) {
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
            Token::String(value) => Expr::string(value.as_str()),
            Token::Symbol(value) => {
                let symbol_str: &str = value.as_str();

                // If `symbol_str` is not an absolute symbol, use the provided `resolver`
                // to attempt to resolve it into a concrete Symbol.
                let symbol = Symbol::try_new(symbol_str).or_else(|| resolver(symbol_str));

                let symbol: Symbol = match symbol {
                    Some(sym) => sym,
                    None => {
                        return Err(Error::custom(format!(
                            "symbol name '{}' has no context",
                            symbol_str
                        )))
                    },
                };

                Expr::symbol(symbol)
            },
            Token::Function { length: arg_count } => {
                drop(value);

                let head = self.get_expr_with_resolver(&mut resolver)?;

                let mut contents = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    contents.push(self.get_expr_with_resolver(&mut resolver)?);
                }

                Expr::normal(head, contents)
            },
        };

        Ok(expr)
    }

    /// Write an expression to this link.
    pub fn put_expr(&mut self, expr: &Expr) -> Result<(), Error> {
        match expr.kind() {
            ExprKind::Normal(normal) => {
                self.put_raw_type(i32::from(sys::WSTKFUNC))?;
                self.put_arg_count(normal.elements().len())?;

                let _: () = self.put_expr(normal.head())?;

                for elem in normal.elements() {
                    let _: () = self.put_expr(elem)?;
                }
            },
            ExprKind::Symbol(symbol) => {
                self.put_symbol(symbol.as_str())?;
            },
            ExprKind::String(string) => {
                self.put_str(string.as_str())?;
            },
            ExprKind::Integer(int) => {
                self.put_i64(*int)?;
            },
            ExprKind::Real(real) => {
                self.put_f64(**real)?;
            },
        }

        Ok(())
    }

    /// Transfer an expression from this link to another.
    ///
    /// # Example
    ///
    /// Transfer an expression between two loopback links:
    ///
    /// ```
    /// use wstp::Link;
    ///
    /// let mut a = Link::new_loopback().unwrap();
    /// let mut b = Link::new_loopback().unwrap();
    ///
    /// // Put an expression into `a`
    /// a.put_i64(5).unwrap();
    ///
    /// // Transfer it to `b`
    /// a.transfer_expr_to(&mut b).unwrap();
    ///
    /// assert_eq!(b.get_i64().unwrap(), 5);
    /// ```
    ///
    /// *WSTP C API Documentation:* [`WSTransferExpression()`](https://reference.wolfram.com/language/ref/c/WSTransferExpression.html)
    pub fn transfer_expr_to(&mut self, dest: &mut Link) -> Result<(), Error> {
        let result = unsafe { sys::WSTransferExpression(dest.raw_link, self.raw_link) };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Transfer the full contents of this loopback link to `dest`.
    ///
    /// *WSTP C API Documentation:* [`WSTransferToEndOfLoopbackLink()`](https://reference.wolfram.com/language/ref/c/WSTransferToEndOfLoopbackLink.html)
    ///
    /// # Panics
    ///
    /// This function will panic if `!self.is_loopback()`.
    pub fn transfer_to_end_of_loopback_link(
        &mut self,
        dest: &mut Link,
    ) -> Result<(), Error> {
        if !self.is_loopback() {
            panic!("transfer_to_end_of_loopback_link(): self must be a loopback link");
        }

        let result =
            unsafe { sys::WSTransferToEndOfLoopbackLink(dest.raw_link, self.raw_link) };

        if result == 0 {
            return if let Some(err) = self.error() {
                Err(err)
            } else if let Some(err) = dest.error() {
                Err(err)
            } else {
                Err(Error::custom("unknown error occurred on WSLINK".into()))
            };
        }

        Ok(())
    }
}

//======================================
// Utilities
//======================================

fn for_each_addr<T, F>(addrs: Vec<net::SocketAddr>, mut func: F) -> Result<T, Error>
where
    F: FnMut(net::SocketAddr) -> Result<T, Error>,
{
    let mut last_error = None;

    for addr in addrs {
        match func(addr) {
            Ok(result) => return Ok(result),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error
        .unwrap_or_else(|| Error::custom(format!("socket address list is empty"))))
}

/// Construct an address string in the special syntax used by WSTP.
fn tcpip_link_name(addr: &net::SocketAddr) -> String {
    format!("{}@{}", addr.port(), addr.ip())
}

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

impl Drop for Link {
    fn drop(&mut self) {
        let Link { raw_link } = *self;

        unsafe {
            sys::WSClose(raw_link);
        }
    }
}
