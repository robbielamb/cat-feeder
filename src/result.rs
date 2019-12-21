pub mod error {
    /// Error enum for this <app_name> app
    #[derive(Debug)]
    pub enum Error {
        //BoringError,
        HyperError(hyper::error::Error),
        HttpError(http::Error),
        TemplateError(askama::Error),
    }

    impl std::error::Error for Error {}

    impl std::convert::From<hyper::error::Error> for Error {
        fn from(err: hyper::error::Error) -> Self {
            Error::HyperError(err)
        }
    }

    impl std::convert::From<http::Error> for Error {
        fn from(err: http::Error) -> Self {
            Error::HttpError(err)
        }
    }

    impl std::convert::From<askama::Error> for Error {
        fn from(err: askama::Error) -> Self {
            Error::TemplateError(err)
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                //Error::BoringError => write!(f, "A boring Error"),
                Error::HyperError(err) => err.fmt(f),
                Error::HttpError(err) => err.fmt(f),
                Error::TemplateError(err) => err.fmt(f),
            }
        }
    }
}

/// Result type for this <app_name> app
pub type Result<T> = std::result::Result<T, error::Error>;
