use std::ffi::CStr;
use std::str::FromStr;

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

    /// Returns the TCPIP port number used by this link server.
    ///
    /// *WSTP C API Documentation:* [WSPortFromLinkServer](https://reference.wolfram.com/language/ref/c/WSPortFromLinkServer.html)
    pub fn port(&self) -> Result<u16, Error> {
        let mut err: std::os::raw::c_int = sys::MLEOK as i32;

        let port: u16 =
            unsafe { sys::WSPortFromLinkServer(self.raw_link_server, &mut err) };

        if err != sys::MLEOK as i32 {
            return Err(Error::from_code(err));
        }

        Ok(port)
    }

    /// Returns the IP address of the interface used by this link server.
    ///
    /// *WSTP C API Documentation:* [WSInterfaceFromLinkServer](https://reference.wolfram.com/language/ref/c/WSInterfaceFromLinkServer.html)
    pub fn interface(&self) -> Result<std::net::IpAddr, Error> {
        let mut err: std::os::raw::c_int = sys::MLEOK as i32;

        let iface_cstr =
            unsafe { sys::WSInterfaceFromLinkServer(self.raw_link_server, &mut err) };


        let iface: String = unsafe {
            let iface = CStr::from_ptr(iface_cstr);

            match iface.to_str() {
                Ok(str) => str.to_string(),
                Err(utf8_error) => {
                    sys::WSReleaseInterfaceFromLinkServer(
                        self.raw_link_server,
                        iface_cstr,
                    );
                    return Err(Error::custom(format!(
                        "LinkServer interface could not be converted to UTF-8 string (error: {}, lossy: '{}')",
                        utf8_error,
                        iface.to_string_lossy()
                    )));
                },
            }
        };

        unsafe {
            sys::WSReleaseInterfaceFromLinkServer(self.raw_link_server, iface_cstr);
        };

        match std::net::IpAddr::from_str(iface.as_str()) {
            Ok(ip) => Ok(ip),
            Err(err) => Err(Error::custom(format!(
                "unable to parse LinkServer interface as IpAddr: {}",
                err
            ))),
        }
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
