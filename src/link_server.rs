use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_int;
use std::str::FromStr;

use crate::{sys, Error, Link};

/// Wrapper around the [`WSLinkServer`](https://reference.wolfram.com/language/ref/c/WSLinkServer.html)
/// C type.
///
/// # Usage
///
/// **TODO:** Document the two different methods for accepting new [`Link`] connections
/// from this type (waiting and an async callback).
pub struct LinkServer {
    raw_link_server: sys::WSLinkServer,
}

/// An iterator that infinitely [`accept`]s connections on a [`LinkServer`].
///
/// This `struct` is created by the [`LinkServer::incoming`] method.
///
/// [`accept`]: LinkServer::accept
pub struct Incoming<'a> {
    server: &'a LinkServer,
}

impl LinkServer {
    /// Create a new `LinkServer` bound to the specified address.
    ///
    /// Use [`Link::connect_to_link_server`] to connect to a `LinkServer`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wstp::LinkServer;
    ///
    /// let server = LinkServer::bind("127.0.0.1:8080").unwrap();
    /// ```
    pub fn bind<A: std::net::ToSocketAddrs>(addrs: A) -> Result<Self, Error> {
        let addrs = addrs.to_socket_addrs().map_err(|err| {
            Error::custom(format!("error binding LinkServer to address: {}", err))
        })?;

        // Try each address, returning the first one which binds successfully.
        crate::for_each_addr(addrs.collect(), |addr| {
            let mut err: std::os::raw::c_int = sys::MLEOK;

            let iface = CString::new(addr.ip().to_string())
                .expect("failed to create CString from LinkServer interface");

            let raw_link_server: sys::WSLinkServer = unsafe {
                sys::WSNewLinkServerWithPortAndInterface(
                    crate::stdenv()?.raw_env,
                    addr.port(),
                    iface.as_ptr(),
                    std::ptr::null_mut(),
                    &mut err,
                )
            };

            if raw_link_server.is_null() || err != sys::MLEOK {
                return Err(Error::from_code(err));
            }

            return Ok(LinkServer { raw_link_server });
        })
    }

    /// Create a new link server.
    ///
    /// It is not possible to register a callback function to accept new link connections
    /// after the link server has been created. Use [`LinkServer::new_with_callback()`] if
    /// that functionality is desired.
    ///
    /// Use [`LinkServer::accept()`] to accept new connections to the link server.
    pub fn new(port: u16) -> Result<Self, Error> {
        let mut err: std::os::raw::c_int = sys::MLEOK;

        let raw_server: sys::WSLinkServer = unsafe {
            sys::WSNewLinkServerWithPort(
                crate::stdenv()?.raw_env,
                port,
                std::ptr::null_mut(),
                &mut err,
            )
        };

        if raw_server.is_null() || err != sys::MLEOK {
            return Err(Error::from_code(err));
        }

        Ok(LinkServer {
            raw_link_server: raw_server,
        })
    }

    /// The callback is required to be [`Send`] so that it can be called from the link
    /// server's background thread, which accepts incoming connections.
    ///
    /// # `'static` bound
    ///
    /// The `'static` bound is required to prevent the callback closure from capturing a
    /// reference to non-static data that it might outlive, for example a local variable:
    ///
    // Note: This example acts as a test that the below code is not possible to write. Do
    //       not remove this example without replacing it with another test.
    /// ```compile_fail
    /// use std::sync::Mutex;
    /// use wstp::LinkServer;
    ///
    /// let mut counter = Mutex::new(0);
    ///
    /// let server = LinkServer::new_with_callback(
    ///     11235,
    ///     // Error: the closure may outlive borrowed value `counter`
    ///     |_| *counter.lock().unwrap() += 1
    /// );
    ///
    /// println!("counter: {}", counter.lock().unwrap());
    /// ```
    ///
    /// Note that the reasoning for the `Send` and `'static` constraints is similiar to
    /// that for [`std::thread::spawn()`], whose documentation may be a useful
    /// additional reference.
    pub fn new_with_callback<F>(port: u16, callback: F) -> Result<Self, Error>
    where
        F: FnMut(Link) + Send + Sync + 'static,
    {
        let mut err: std::os::raw::c_int = sys::MLEOK;

        let raw_server: sys::WSLinkServer = unsafe {
            sys::WSNewLinkServerWithPort(
                crate::stdenv()?.raw_env,
                port,
                Box::into_raw(Box::new(callback)) as *mut std::ffi::c_void,
                &mut err,
            )
        };

        if raw_server.is_null() || err != sys::MLEOK {
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
    pub fn port(&self) -> u16 {
        self.try_port()
            .unwrap_or_else(|err| panic!("WSPortFromLinkServer failed: {}", err))
    }

    /// Fallible variant of [LinkServer::port()].
    pub fn try_port(&self) -> Result<u16, Error> {
        let mut err: std::os::raw::c_int = sys::MLEOK;

        let port: u16 =
            unsafe { sys::WSPortFromLinkServer(self.raw_link_server, &mut err) };

        if err != sys::MLEOK {
            return Err(Error::from_code(err));
        }

        Ok(port)
    }

    /// Returns the IP address of the interface used by this link server.
    ///
    /// *WSTP C API Documentation:* [WSInterfaceFromLinkServer](https://reference.wolfram.com/language/ref/c/WSInterfaceFromLinkServer.html)
    pub fn interface(&self) -> std::net::IpAddr {
        self.try_interface()
            .unwrap_or_else(|err| panic!("WSInterfaceFromLinkServer failed: {}", err))
    }

    /// Fallible variant of [LinkServer::interface()].
    pub fn try_interface(&self) -> Result<std::net::IpAddr, Error> {
        let mut err: c_int = sys::MLEOK;

        let iface_cstr =
            unsafe { sys::WSInterfaceFromLinkServer(self.raw_link_server, &mut err) };

        if iface_cstr.is_null() || err != sys::MLEOK {
            return Err(Error::from_code(err));
        }

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
                "unable to parse LinkServer interface ({}) as IpAddr: {}",
                iface, err
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

    /// Accept a new incoming connection to this link server.
    ///
    /// This method blocks the current thread indefinitely until a connection is made to
    /// the port this link server is bound to.
    ///
    /// Use [`LinkServer::new_with_callback()`] to create a link server which accepts
    /// connections asyncronously via a callback function.
    ///
    /// *WSTP C API Documentation:* [`WSWaitForNewLinkFromLinkServer`](https://reference.wolfram.com/language/ref/c/WSWaitForNewLinkFromLinkServer.html)
    pub fn accept(&self) -> Result<Link, Error> {
        let mut err: c_int = sys::MLEOK;

        let raw_link = unsafe {
            sys::WSWaitForNewLinkFromLinkServer(self.raw_link_server, &mut err)
        };

        if raw_link.is_null() || err != sys::MLEOK {
            return Err(Error::from_code(err));
        }

        let link = unsafe { Link::unchecked_new(raw_link) };

        Ok(link)
    }

    /// Returns an iterator over the connections being received on this server.
    ///
    /// The returned iterator will never return None. Iterating over it is equivalent to
    /// calling [`LinkServer::accept`] in a loop.
    pub fn incoming(&self) -> Incoming {
        Incoming { server: self }
    }

    /// Returns the raw [`WSLinkServer`](https://reference.wolfram.com/language/ref/c/WSLinkServer.html)
    /// C type wrapped by this [`LinkServer`].
    pub fn raw_link_server(&self) -> sys::WSLinkServer {
        self.raw_link_server
    }
}

extern "C" fn callback_trampoline<F: FnMut(Link) + Send + Sync + 'static>(
    raw_link_server: sys::WSLinkServer,
    raw_link: sys::WSLINK,
) {
    let mut err: std::os::raw::c_int = sys::MLEOK;

    let user_closure: &mut F;
    let link: Link;

    unsafe {
        let raw_user_closure: *mut std::ffi::c_void =
            sys::WSContextFromLinkServer(raw_link_server, &mut err);

        user_closure = &mut *(raw_user_closure as *mut F);

        // SAFETY: This is safe because `raw_link` is an entirely new link which we have
        //         ownership over.
        link = Link::unchecked_new(raw_link);
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

impl fmt::Debug for LinkServer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LinkServer(Port: {}, Interface: {})",
            self.port(),
            self.interface()
        )
    }
}

impl<'a> Iterator for Incoming<'a> {
    type Item = Result<Link, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.server.accept())
    }
}
