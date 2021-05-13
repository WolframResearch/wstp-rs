use std::fmt::{self, Debug, Display};

pub struct Error {
    pub(crate) message: String,
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
