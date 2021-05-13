use crate::{sys, Error, WstpEnv, WstpLink};

/// Wrapper around the [`WSLinkServer`](https://reference.wolfram.com/language/ref/c/WSLinkServer.html)
/// C type.
///
/// # Usage
///
/// **TODO:** Document the two different methods for accepting new `WstpLink` connections
/// from this type (waiting and an async callback).
#[derive(Debug)]
pub struct LinkServer {
    raw_link_server: sys::WSLinkServer,
}

impl LinkServer {
    /// The callback is required to be [`Send`] so that it can be called from the link
    /// server's background thread, which accepts incoming connections.
    pub fn new_with_callback<F>(
        env: &WstpEnv,
        port: u16,
        callback: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(WstpLink) + Send + Sync,
    {
        let raw_server: sys::WSLinkServer;
        let mut err: std::os::raw::c_int = sys::MLEOK as i32;

        unsafe {
            raw_server = sys::WSNewLinkServerWithPort(
                env.raw_env,
                port,
                Box::into_raw(Box::new(callback)) as *mut std::ffi::c_void,
                &mut err,
            );
        }

        if raw_server.is_null() || err != (sys::MLEOK as i32) {
            return Err(Error::from_code(err));
        }

        unsafe {
            sys::WSRegisterCallbackFunctionWithLinkServer(
                raw_server,
                Some(callback_trampoline::<F>),
            )
        }

        Ok(LinkServer {
            raw_link_server: raw_server,
        })
    }

    /// Close this link server.
    ///
    /// This link server will stop accepting new connections, and unbind from the network
    /// port it is attached to.
    ///
    /// *WSTP C API Documentation:* [`WSShutdownLinkServer`](https://reference.wolfram.com/language/ref/c/WSShutdownLinkServer.html)
    pub fn close(self) {
        // Note: The link server is closed when `self` is dropped.
    }

    /// Returns the raw [`WSLinkServer`](https://reference.wolfram.com/language/ref/c/WSLinkServer.html)
    /// C type wrapped by this [`LinkServer`].
    pub fn raw_link_server(&self) -> sys::WSLinkServer {
        self.raw_link_server
    }
}

extern "C" fn callback_trampoline<F: FnMut(WstpLink)>(
    raw_link_server: sys::WSLinkServer,
    raw_link: sys::WSLINK,
) {
    let mut err: std::os::raw::c_int = sys::MLEOK as i32;

    let user_closure: &mut F;
    let link: WstpLink;

    unsafe {
        let raw_user_closure: *mut std::ffi::c_void =
            sys::WSContextFromLinkServer(raw_link_server, &mut err);

        user_closure = &mut *(raw_user_closure as *mut F);

        link = WstpLink::unchecked_new(raw_link);
    }

    // Call the closure provided by the user
    // FIXME: Catch panic's in the user's code to prevent unwinding over C stack frames.
    user_closure(link);
}

impl Drop for LinkServer {
    fn drop(&mut self) {
        let LinkServer { raw_link_server } = *self;

        unsafe {
            sys::WSShutdownLinkServer(raw_link_server);
        }
    }
}
