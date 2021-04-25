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

    /// The control archive was already read and thus can not be read again
    ControlAlreadyRead,

    /// The data archive was already read and thus can not be read again
    DataAlreadyRead,

    /// The entry in the deb package was an unknown file format
    UnknownEntryFormat,

    /// These was an IoError during the parsing
    Io(IoError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidVersion => write!(f, "Contents of debian_binary is not 2.x"),
            Error::MissingDebianBinary => write!(f, "Missing debian_binary file"),
            Error::MissingControlFile => write!(f, "control archive is missing control file"),
            Error::MissingPackageName => write!(f, "control file did not contain a package name"),
            Error::MissingPackageVersion => {
                write!(f, "control file did not contain a package version")
            }
            Error::InvalidControlFile => write!(f, "control file missed formatted"),
            Error::MissingControlArchive => write!(f, "control archive is missing"),
            Error::MissingDataArchive => write!(f, "data archive is missing"),
            Error::ControlAlreadyRead => write!(f, "control archive has been past"),
            Error::DataAlreadyRead => write!(f, "data archive has been past"),
            Error::UnknownEntryFormat => {
                write!(f, "entry in debian package has unknown file format")
            }
            Error::Io(ref err) => write!(f, "{}", err),
        }
    }
}

impl StdError for Error {
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
