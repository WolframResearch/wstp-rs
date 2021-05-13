use std::fmt::{self, Debug, Display};

#[derive(Clone, PartialEq)]
pub struct Error {
    pub(crate) code: Option<i32>,
    pub(crate) message: String,
}

impl Error {
    /// Get the WSTP error code, if applicable.
    ///
    /// See listing of WSTP error codes in the [`WSError()`](https://reference.wolfram.com/language/ref/c/WSError.html)
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
        // TODO: Map this to known error codes, provide a better string.
        Error {
            code: Some(code),
            message: format!("WSTP error code {} occurred.", code),
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
