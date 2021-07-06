//! WSTP environment object management.
//!
//! It's necessary that a `WSENV` always outlive any links which are created in
//! that environment. However, requiring that every [`WstpLink`][crate::WstpLink] be tied
//! to the lifetime of a [`WstpEnv`] created by the user would make the `wstp` API
//! unnecessarily burdensome. The easiest way to manage this is to have a single,
//! global, shared environment instance, and use that internally in every `wstp`
//! wrapper API. (This is what [`stdenv`](https://reference.wolfram.com/language/ref/c/stdenv.html)
//! accomplishes for programs prepared with [`wsprep`](https://reference.wolfram.com/language/ref/program/wsprep.html)).
//!
//! In general, the existence of an explicit, shared WSTP environment object is a bit of
//! an anachronism -- ideally it wouldn't exist at all. Much of what `WSENV` contains is
//! effectively global state (e.g. signal handlers), which might better be represented as
//! hidden global variables in the WSTP C library. Where possible, `wstp` should avoid
//! exposing this detail of the WSTP C API.
//!
//! # Safety
//!
//! If the determination is made in the future to expose [`WstpEnv`] publically from `wstp`,
//! some safety conditions will need to be satisfied:
//!
//!   * A `WstpLink` MUST NOT be able to outlive the `WstpEnv` which it's creation was
//!     associated with.
//!   * All `WstpLink`'s MUST be closed before the `WstpEnv` they are associated with is
//!     deinitialized (essentially a restatement of the first condition).

use std::sync::Mutex;

use crate::{sys, Error};

lazy_static::lazy_static! {
    /// The standard WSTP environment object.
    ///
    /// *WSTP C API Documentation:* [`stdenv`](https://reference.wolfram.com/language/ref/c/stdenv.html)
    static ref STDENV: Mutex<WstpEnv> = {
        Mutex::new(initialize().unwrap())
    };
}
/// Private. A WSTP library environment.
///
/// NOTE: This function should remain private. See note on [`crate::env`].
///
/// See [`initialize()`].
///
/// *WSTP C API Documentation:* [`WSENV`](https://reference.wolfram.com/language/ref/c/WSENV.html).
pub(crate) struct WstpEnv {
    pub raw_env: sys::WSENV,
}

unsafe impl Send for WstpEnv {}

/// Private.
///
/// NOTE: This function should remain private. See note on [`crate::env`].
///
/// *WSTP C API Documentation:* [`WSInitialize()`](https://reference.wolfram.com/language/ref/c/WSInitialize.html)
fn initialize() -> Result<WstpEnv, Error> {
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
    #[allow(dead_code)]
    pub fn raw_env(&self) -> sys::WSENV {
        let WstpEnv { raw_env } = *self;

        raw_env
    }
}

/// Acquire a lock on [`struct@STDENV`].
pub(crate) fn stdenv() -> Result<std::sync::MutexGuard<'static, WstpEnv>, Error> {
    STDENV.lock().map_err(|err| {
        Error::custom(format!("Unable to acquire lock on STDENV: {}", err))
    })
}

impl Drop for WstpEnv {
    fn drop(&mut self) {
        let WstpEnv { raw_env } = *self;

        unsafe {
            sys::WSDeinitialize(raw_env);
        }
    }
}
