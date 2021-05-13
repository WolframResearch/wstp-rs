use std::fmt::{self, Debug, Display};

pub struct Error {
    pub(crate) message: String,
}

impl Error {
    pub(crate) fn from_code(code: i32) -> Self {
        // TODO: Map this to known error codes, provide a better string.
        Error {
            message: format!("WSTP error code {} occurred.", code),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Error { message } = self;

        write!(f, "{}", message)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Any further information we could provide here?
        write!(f, "{}", self)
    }
}
