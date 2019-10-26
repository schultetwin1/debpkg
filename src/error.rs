use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    InvalidVersion,
    MissingDebianBinary,
    MissingControlFile,
    MissingPackageName,
    InvalidPackageType,
    MissingControlArchive,
    MissingDataArchive,
    UnknownControlField,
    EmptyArchive,
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
            Error::InvalidVersion => "Contents of debian_binary is not 2.0",
            Error::MissingDebianBinary => "Missing debian_binary file",
            Error::EmptyArchive => "Archive is empty",
            Error::MissingControlFile => "control archive is missing control file",
            Error::MissingPackageName => "control file did not contain a package name",
            Error::InvalidPackageType => "control file contained an invalid package type",
            Error::MissingControlArchive => "control archive is missing",
            Error::MissingDataArchive => "data archive is missing",
            Error::UnknownControlField => "control field is unknown",
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
