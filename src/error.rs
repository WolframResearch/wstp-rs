use std::{
    ffi::CStr,
    fmt::{self, Debug, Display},
    os::raw::c_char,
};

/// WSTP link error.
///
/// Use [`Error::code()`] to retrieve the WSTP error code, if applicable.
#[derive(Clone, PartialEq)]
pub struct Error {
    pub(crate) code: Option<i32>,
    pub(crate) message: String,
}

impl Error {
    /// Get the WSTP error code, if applicable.
    ///
    /// Possible error codes are listed in the [`WSError()`](https://reference.wolfram.com/language/ref/c/WSError.html)
    /// documentation.
    pub fn code(&self) -> Option<i32> {
        self.code
    }

    pub(crate) fn custom(message: String) -> Self {
        Error {
            code: None,
            message,
        }
    }

    pub(crate) fn from_code(code: i32) -> Self {
        // Lookup the error string describing this error code.
        let message: String = crate::env::stdenv()
            .ok()
            .and_then(|stdenv| unsafe {
                // Note: We do not need to free this, because it's scoped to our eternal
                //       STDENV instance.
                let message_cptr: *const c_char =
                    crate::sys::WSErrorString(stdenv.raw_env, i64::from(code));

                if message_cptr.is_null() {
                    return None;
                }

                let message_cstr = CStr::from_ptr(message_cptr);

                Some(message_cstr.to_str().ok()?.to_owned())
            })
            .unwrap_or_else(|| format!("WSTP error code {} occurred.", code));

        Error {
            code: Some(code),
            message,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Error { code, message } = self;

        if let Some(code) = code {
            write!(f, "WSTP error (code {}): {}", code, message)
        } else {
            write!(f, "WSTP error: {}", message)
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Any further information we could provide here?
        write!(f, "{}", self)
    }
}

impl std::error::Error for Error {}
