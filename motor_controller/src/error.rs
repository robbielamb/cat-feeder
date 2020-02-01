use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Errors that occur with the PCA chip
    PCA9685Error(pca9685_pwc::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::PCA9685Error(ref error) => error.fmt(f),
        }
    }
}

impl error::Error for Error {}

impl From<pca9685_pwc::Error> for Error {
    fn from(err: pca9685_pwc::Error) -> Error {
        Error::PCA9685Error(err)
    }
}
