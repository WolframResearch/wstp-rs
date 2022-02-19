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
//! [WolframKernel]: https://reference.wolfram.com/language/ref/program/WolframKernel.html

use std::{path::PathBuf, process};

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
        // FIXME: Make this a random string.
        const NAME: &str = "SHM_WK_LINK";

        let listener = std::thread::spawn(|| {
            // This will block until a connection is made.
            Link::listen(Protocol::SharedMemory, NAME)
        });

        let kernel_process = process::Command::new(path)
            .arg("-wstp")
            .arg("-linkprotocol")
            .arg("SharedMemory")
            .arg("-linkconnect")
            .arg("-linkname")
            .arg(NAME)
            .spawn()?;

        let link: Link = match listener.join() {
            Ok(result) => result?,
            Err(panic) => {
                return Err(Error(format!(
                    "unable to launch Wolfram Kernel: listening thread panicked: {:?}",
                    panic
                )))
            },
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
