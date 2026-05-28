//! Utilities for interacting with a Wolfram Kernel process via WSTP.
//!
//! # Example
//!
//! Launch a new Wolfram Kernel process from the file path to a
//! [`WolframKernel`][WolframKernel] executable:
//!
//! ```no_run
//! use std::path::PathBuf;
//! use wstp::kernel::WolframKernelProcess;
//!
//! let exe = PathBuf::from(
//!     "/Applications/Mathematica.app/Contents/MacOS/WolframKernel"
//! );
//!
//! let kernel = WolframKernelProcess::launch(&exe).unwrap();
//! ```
//!
//! ### Automatic Wolfram Kernel discovery
//!
//! Use the [wolfram-app-discovery] crate to automatically discover a suitable
//! `WolframKernel`:
//!
//! ```no_run
//! use std::path::PathBuf;
//! use wolfram_app_discovery::WolframApp;
//! use wstp::kernel::WolframKernelProcess;
//!
//! let app = WolframApp::try_default()
//!     .expect("unable to find any Wolfram Language installations");
//!
//! let exe: PathBuf = app.kernel_executable_path().unwrap();
//!
//! let kernel = WolframKernelProcess::launch(&exe).unwrap();
//! ```
//!
//! Using automatic discovery makes it easy to write programs that are portable to
//! different computers, without relying on end-user configuration to specify the location
//! of the local Wolfram Language installation.
//!
//!
//! [WolframKernel]: https://reference.wolfram.com/language/ref/program/WolframKernel.html
//! [wolfram-app-discovery]: https://crates.io/crates/wolfram-app-discovery
//!
//!
//! # Related Links
//!
//! #### Wolfram Language documentation
//!
//! These resources describe the packet expression interface used by the Wolfram Kernel.
//!
//! * [WSTP Packets](https://reference.wolfram.com/language/guide/WSTPPackets.html)
//! * [Running the Wolfram System from within an External Program](https://reference.wolfram.com/language/tutorial/RunningTheWolframSystemFromWithinAnExternalProgram.html)
//!
//! #### Link packet methods
//!
//! * [`Link::put_eval_packet()`]

use std::{path::PathBuf, process, time::Duration};

use wolfram_expr::Expr;

use crate::{Error as WstpError, Link, Protocol};

/// Handle to a Wolfram Kernel process connected via WSTP.
///
/// Use [`WolframKernelProcess::launch()`] to launch a new Wolfram Kernel process.
///
/// Use [`WolframKernelProcess::link()`] to access the WSTP [`Link`] used to communicate with
/// this kernel.
#[derive(Debug)]
pub struct WolframKernelProcess {
    #[allow(dead_code)]
    process: process::Child,
    link: Link,
}

/// Wolfram Kernel process error.
#[derive(Debug)]
pub struct Error(String);

impl From<WstpError> for Error {
    fn from(err: WstpError) -> Error {
        Error(format!("WSTP error: {err}"))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error(format!("IO error: {err}"))
    }
}

impl WolframKernelProcess {
    /// Launch a new Wolfram Kernel child process and establish a WSTP connection with it.
    ///
    /// See also the [wolfram-app-discovery](https://crates.io/crates/wolfram-app-discovery)
    /// crate, whose
    /// [`WolframApp::kernel_executable_path()`](https://docs.rs/wolfram-app-discovery/0.2.0/wolfram_app_discovery/struct.WolframApp.html#method.kernel_executable_path)
    /// method can be used to get the location of a [`WolframKernel`][WolframKernel]
    /// executable suitable for use with this function.
    ///
    /// [WolframKernel]: https://reference.wolfram.com/language/ref/program/WolframKernel.html
    //
    // TODO: Would it be correct to describe this as essentially `LinkLaunch`? Also note
    //       that this doesn't actually use `-linkmode launch`.
    pub fn launch(path: &PathBuf) -> Result<WolframKernelProcess, Error> {
        Self::launch_impl(path, None)
    }

    /// Like [`WolframKernelProcess::launch()`], but aborts with an error if the
    /// spawned kernel does not complete the WSTP handshake within `timeout`.
    ///
    /// Use this when you want to bound how long the caller blocks waiting for a
    /// kernel that may fail to start (e.g. licensing failures, missing binary,
    /// misconfigured environment). The underlying mechanism is
    /// [`Link::activate_with_timeout()`].
    pub fn launch_with_timeout(
        path: &PathBuf,
        timeout: Duration,
    ) -> Result<WolframKernelProcess, Error> {
        Self::launch_impl(path, Some(timeout))
    }

    fn launch_impl(
        path: &PathBuf,
        timeout: Option<Duration>,
    ) -> Result<WolframKernelProcess, Error> {
        let mut link = Link::listen(Protocol::SharedMemory, "")?;

        let name = link.link_name();
        assert!(!name.is_empty());

        let kernel_process = process::Command::new(path)
            .arg("-wstp")
            .arg("-linkprotocol")
            .arg("SharedMemory")
            .arg("-linkconnect")
            .arg("-linkname")
            .arg(&name)
            .spawn()?;

        // Wait for an incoming connection to be made to the listening link.
        // With `timeout = None` this blocks forever (matching the historical
        // `launch()` behaviour). With `Some(d)` the wait is bounded by the
        // cooperative yield-function abort installed by
        // `Link::activate_with_timeout`.
        match timeout {
            Some(d) => link.activate_with_timeout(d)?,
            None => link.activate()?,
        };

        Ok(WolframKernelProcess {
            process: kernel_process,
            link,
        })
    }

    /// Get the WSTP [`Link`] connection used to communicate with this Wolfram Kernel
    /// process.
    pub fn link(&mut self) -> &mut Link {
        let WolframKernelProcess { process: _, link } = self;
        link
    }
}

impl Link {
    /// Put an [`EvaluatePacket[expr]`][EvaluatePacket] onto the link.
    ///
    /// [EvaluatePacket]: https://reference.wolfram.com/language/ref/EvaluatePacket.html
    pub fn put_eval_packet(&mut self, expr: &Expr) -> Result<(), Error> {
        self.put_function("System`EvaluatePacket", 1)?;
        self.put_expr(expr)?;
        self.end_packet()?;

        Ok(())
    }
}
