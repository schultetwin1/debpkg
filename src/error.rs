use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
/// Errors from parsing Debian packages
pub enum Error {
    /// The debian package in not version 2.x
    InvalidVersion,

    /// The ar archive does not contain the "debian_binary" file
    MissingDebianBinary,

    /// The conrtol archive does not contain a control file
    MissingControlFile,

    /// The control file does not contain a package name
    MissingPackageName,

    /// The control file does not contain a package version
    MissingPackageVersion,

    /// The control file is not formatted correctly
    InvalidControlFile,

    /// The ar archive does not contain a control archive
    MissingControlArchive,

    /// The ar archive does not contain a data archive
    MissingDataArchive,

    /// These was an IoError during the parsing
    Io(IoError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "{}", err),
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::InvalidVersion => "Contents of debian_binary is not 2.x",
            Error::MissingDebianBinary => "Missing debian_binary file",
            Error::MissingControlFile => "control archive is missing control file",
            Error::MissingPackageName => "control file did not contain a package name",
            Error::MissingPackageVersion => "control file did not contain a package version",
            Error::InvalidControlFile => "control file missed formatted",
            Error::MissingControlArchive => "control archive is missing",
            Error::MissingDataArchive => "data archive is missing",
            Error::Io(_err) => "IO Error",
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            Error::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}
