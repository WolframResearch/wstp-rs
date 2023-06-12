//! WSTP environment object management.
//!
//! It's necessary that a `WSENV` always outlive any links which are created in
//! that environment. However, requiring that every [`Link`][crate::Link] be tied
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
//!   * A [`Link`][crate::Link] MUST NOT be able to outlive the `WstpEnv` that its
//!     creation was associated with.
//!   * All [`Link`][crate::Link]'s MUST be closed before the `WstpEnv` they are
//!     associated with is deinitialized (essentially a restatement of the first condition).

use std::{
    ops::Deref,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::Lazy;

use crate::{sys, Error};

/// The standard WSTP environment object.
///
/// *WSTP C API Documentation:* [`stdenv`](https://reference.wolfram.com/language/ref/c/stdenv.html)
static STDENV: Lazy<Mutex<Option<WstpEnv>>> =
    Lazy::new(|| Mutex::new(Some(initialize().unwrap())));

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

/// An RAII guard that provides scoped access to the `STDENV` static.
pub(crate) struct StdEnv {
    guard: MutexGuard<'static, Option<WstpEnv>>,
}

impl Deref for StdEnv {
    type Target = WstpEnv;

    fn deref(&self) -> &WstpEnv {
        match self.guard.as_ref() {
            Some(stdenv) => stdenv,
            None => panic!("STDENV has been shutdown."),
        }
    }
}

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


/// Deinitialize the [`WSENV`] static maintained by this library.
///
/// Ideally, this function would not be necessary. However, the WSTP C library internally
/// launches several background threads necessary for its operation. If these threads are
/// still running when the main() function returns, an ungraceful shutdown can occur, with
/// error messages being printed. This function is an escape hatch to permit users of this
/// library to ensure that all background thread shutdown before `main()` returns.
///
/// TODO: Make this function obsolete, either by changing the WSTP C library
///       implementation, or, perhaps easier, maintain a reference count of the number of
///       [`Link`] objects that have been created, and (re-)initialize and deinitialize
///       the `WSENV` static whenever that count rises from or falls to 0.
///
/// # Safety
///
/// All [`Link`] objects created by this library are associated with the global [`WSENV`]
/// static used internally. Deinitializing the global `WSENV` before all [`Link`] objects
/// have been dropped is not legal. Only call this function after ensuring that all
/// [`Link`] objects created by your code have been dropped.
#[doc(hidden)]
pub unsafe fn shutdown() -> Result<bool, Error> {
    let mut guard = STDENV.lock().map_err(|err| {
        Error::custom(format!("Unable to acquire lock on STDENV: {}", err))
    })?;

    let was_initialized = if let Some(env) = guard.take() {
        env.deinitialize();
        true
    } else {
        false
    };

    Ok(was_initialized)
}

impl WstpEnv {
    #[allow(dead_code)]
    pub fn raw_env(&self) -> sys::WSENV {
        let WstpEnv { raw_env } = *self;

        raw_env
    }

    fn deinitialize(self) {
        let WstpEnv { raw_env } = self;

        unsafe { sys::WSDeinitialize(raw_env) }
    }
}

/// Acquire a lock on [`struct@STDENV`].
pub(crate) fn stdenv() -> Result<StdEnv, Error> {
    let guard = STDENV.lock().map_err(|err| {
        Error::custom(format!("Unable to acquire lock on STDENV: {}", err))
    })?;

    Ok(StdEnv { guard })
}

impl Drop for WstpEnv {
    fn drop(&mut self) {
        let WstpEnv { raw_env } = *self;

        unsafe {
            sys::WSDeinitialize(raw_env);
        }
    }
}
