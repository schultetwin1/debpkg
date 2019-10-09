use std::io::{Read, Seek};
use std::io::Error as IoError;
use std::string::String;
use std::fmt;
use std::error::Error as StdError;


#[derive(Debug)]
pub enum Error {
    InvalidVersion,
    MissingDebianBinary,
    MissingControlFile,
    MissingControlArchive,
    MissingDataArchive,
    EmptyArchive,
    Io(IoError),
    LzmaError(lzma::LzmaError)
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
            Error::MissingControlArchive => "control archive is missing",
            Error::MissingDataArchive => "data archive is missing",
            Error::Io(_err) => "IO Error",
            Error::LzmaError(_err) => "Lzma Error",
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::LzmaError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl From<lzma::LzmaError> for Error {
    fn from(err: lzma::LzmaError) -> Error {
        Error::LzmaError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct DebPkg<R: Seek + Read> {
    archive: ar::Archive<R>
}

fn check_debian_binary_contents<R: Read>(entry: &mut ar::Entry<R>) -> Result<()> {
    let mut contents: String = String::new();
    entry.read_to_string(&mut contents)?;

    if contents == "2.0\n" {
        Ok(())
    } else {
        Err(Error::InvalidVersion)
    }
}

fn check_for_debian_binary<R: Read>(archive: &mut ar::Archive<R>) -> Result<()> {
    let identifier = "debian-binary";
    if let Some(entry_result) = archive.next_entry() {
        match entry_result {
            Ok(mut entry) => {
                if entry.header().identifier() == identifier.as_bytes() {
                    check_debian_binary_contents(&mut entry)?;
                } else {
                    return Err(Error::MissingDebianBinary);
                }
            },
            Err(err) => return Err(Error::Io(err))
        }
    } else {
        return Err(Error::EmptyArchive);
    }
    Ok(())
}

fn untar_control_data<R: Read>(tar_reader: R) -> Result<String> {
    let mut tar = tar::Archive::new(tar_reader);
    let entries = tar.entries()?;
    let control_entry = entries.filter_map(|x| x.ok()).filter(|entry| entry.path().is_ok()).find(|entry| {
        let path = entry.path().unwrap();
        path == std::path::Path::new("./control")
    });
    match control_entry {
        Some(mut control) => {
            let mut string = std::string::String::default();
            let _ = control.read_to_string(&mut string)?;
            Ok(string)
        },
        None => Err(Error::MissingControlFile)
    }
}

fn extract_control_data<R: Read>(archive: &mut ar::Archive<R>) -> Result<String> {
    if let Some(entry_result) = archive.next_entry() {
        match entry_result {
            Ok(entry) => {
                let entry_ident = std::str::from_utf8(entry.header().identifier()).unwrap();

                match entry_ident {
                    "control.tar" => {
                        untar_control_data(entry)
                    },
                    "control.tar.gz" => {
                        let reader = flate2::read::GzDecoder::new(entry);
                        untar_control_data(reader)
                    },
                    "control.tar.xz" => {
                        let reader = lzma::LzmaReader::new_decompressor(entry)?;
                        untar_control_data(reader)
                    },
                    "control.tar.zst" => unimplemented!(),
                    _ => {
                        Err(Error::MissingControlArchive)
                    }
                }
            },
            Err(err) => {
                Err(Error::Io(err))
            }
        }
    } else {
        Err(Error::MissingControlArchive)
    }
}

fn get_data_entry<R: Read>(archive: &mut ar::Archive<R>) -> Result<ar::Entry<R>> {
    if let Some(entry_result) = archive.next_entry() {
        match entry_result {
            Ok(entry) => {
                let entry_ident = std::str::from_utf8(entry.header().identifier()).unwrap();

                match entry_ident {
                    "data.tar" => {
                        Ok(entry)
                    },
                    "data.tar.gz" => {
                        Ok(entry)
                    },
                    "data.tar.xz" => {
                        Ok(entry)
                    },
                    "data.tar.zst" => unimplemented!(),
                    _ => {
                        Err(Error::MissingDataArchive)
                    }
                }
            },
            Err(err) => {
                Err(Error::Io(err))
            }
        }
    } else {
        Err(Error::MissingDataArchive)
    }

}

impl<R: Read + Seek> DebPkg<R> {
    pub fn parse(reader: R) -> Result<DebPkg<R>> {
        let mut archive = ar::Archive::new(reader);

        check_for_debian_binary(&mut archive)?;
        let _control = extract_control_data(&mut archive)?;
        let _ = get_data_entry(&mut archive)?;

        Ok(DebPkg {
            archive
        })
    }

    pub fn unpack<P: AsRef<std::path::Path>>(&mut self, dst: P) -> Result<()> {
        let entry = self.archive.jump_to_entry(2)?;
        let entry_ident = std::str::from_utf8(entry.header().identifier()).unwrap();

        match entry_ident {
            "data.tar" => {
                let mut tar = tar::Archive::new(entry);
                tar.unpack(dst)?;
                Ok(())
            },
            "data.tar.gz" => {
                let gz = flate2::read::GzDecoder::new(entry);
                let mut tar = tar::Archive::new(gz);
                tar.unpack(dst)?;
                Ok(())
            },
            "data.tar.xz" => {
                let xz = lzma::LzmaReader::new_decompressor(entry)?;
                let mut tar = tar::Archive::new(xz);
                tar.unpack(dst)?;
                Ok(())
            },
            "data.tar.zst" => unimplemented!(),
            _ => {
                Err(Error::MissingDataArchive)
            }
        }

    }
}

#[cfg(test)]
mod tests {
}
