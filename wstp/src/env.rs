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

use std::sync::Mutex;

use crate::{sys, Error};

/// The standard WSTP environment object.
///
/// *WSTP C API Documentation:* [`stdenv`](https://reference.wolfram.com/language/ref/c/stdenv.html)
static STDENV: Mutex<StdEnvState> = Mutex::new(StdEnvState::Uninitialized);

enum StdEnvState {
    /// No links have been created yet, so the lazily initialized STDENV is
    /// empty.
    Uninitialized,
    Initialized(WstpEnv),
    /// The WSTP library was shutdown so the STDENV was deinitialized and cannot
    /// be re-initialized.
    Shutdown,
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

/// FIXME: This is only valid for [`STDENV`] because we enforce exclusive access
///        via [`with_raw_stdenv()`]. Other general instances of `WstpEnv`
///        are not safe to send between threads. Use ForceSend?
unsafe impl Send for WstpEnv {}

/// Enforce unique access to the raw `STDENV` value.
///
/// This prevents trying to create links stored on the same global `WSENV`
/// instance in multiple threads at the same time. The `WSENV` type and WSTP API
/// functions do not otherwise do synchronization when mutating `WSENV` instances.
pub(crate) fn with_raw_stdenv<T, F: FnOnce(sys::WSENV) -> T>(
    callback: F,
) -> Result<T, Error> {
    let mut guard = STDENV.lock().map_err(|err| {
        Error::custom(format!("Unable to acquire lock on STDENV: {}", err))
    })?;

    if let StdEnvState::Uninitialized = *guard {
        *guard = StdEnvState::Initialized(WstpEnv::initialize().unwrap())
    }

    let raw_env = match &*guard {
        StdEnvState::Uninitialized => unreachable!(),
        StdEnvState::Initialized(stdenv) => stdenv.raw_env,
        StdEnvState::Shutdown => {
            return Err(Error::custom(
                "wstp-rs: STDENV has been shutdown. No more links can be created."
                    .to_owned(),
            ))
        },
    };

    // Call the callback during the period that we hold `guard`.
    let result = callback(raw_env);

    drop(guard);

    Ok(result)
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

    // Take the current state and set STDENV to Shutdown.
    let state = std::mem::replace(&mut *guard, StdEnvState::Shutdown);

    let was_initialized = match state {
        StdEnvState::Uninitialized => false,
        StdEnvState::Initialized(stdenv) => {
            stdenv.deinitialize();
            true
        },
        // TODO(cleanup): Should this panic instead? shutdown() shouldn't be
        //                called more than once.
        StdEnvState::Shutdown => false,
    };

    Ok(was_initialized)
}

impl WstpEnv {
    /// Private.
    ///
    /// NOTE: This function should remain private. See note on [`crate::env`].
    ///
    /// *WSTP C API Documentation:* [`WSInitialize()`](https://reference.wolfram.com/language/ref/c/WSInitialize.html)
    pub(crate) fn initialize() -> Result<Self, Error> {
        // TODO: Is this thread-safe?
        //       Is it safe to call WSInitialize() multiple times in the same process?
        let raw_env: sys::WSENV = unsafe { sys::WSInitialize(std::ptr::null_mut()) };

        if raw_env.is_null() {
            return Err(Error::custom(
                // TODO: Is there an internal error string which could be included here?
                format!("WSInitialize() failed"),
            ));
        }

        Ok(WstpEnv { raw_env })
    }

    #[allow(dead_code)]
    pub(crate) fn raw_env(&self) -> sys::WSENV {
        let WstpEnv { raw_env } = *self;

        raw_env
    }

    fn deinitialize(self) {
        drop(self)
    }
}

impl Drop for WstpEnv {
    fn drop(&mut self) {
        let WstpEnv { raw_env } = *self;

        unsafe {
            sys::WSDeinitialize(raw_env);
        }
    }
}
