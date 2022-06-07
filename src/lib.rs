//! # debpkg
//!
//! A library to parse binary debian packages
//!
//! This library provides utilties to parse [binary debian
//! packages](https://www.debian.org/doc/manuals/debian-faq/ch-pkg_basics.en.html#s-deb-format)
//! abstracted over a reader. This API provides a streaming interface to avoid
//! loading the entire debian package into RAM.
//!
//! This library only parses binary debian packages. It does not attempt to
//! write binary debian packages.
//!
//! # Supported Debian Package Versions
//!
//! This package only supports version 2.0 of debian packages. Older versions
//! are not currently supported.
//!
//! # Examples
//!
//! Parsing a debian package
//!
//! ```no_run
//! let file = std::fs::File::open("test.deb").unwrap();
//! let mut pkg = debpkg::DebPkg::parse(file).unwrap();
//! let mut control_tar = pkg.control().unwrap();
//! let control = debpkg::Control::extract(control_tar).unwrap();
//! println!("Package Name: {}", control.name());
//! println!("Package Version: {}", control.version());
//! let arch = control.get("Architecture").unwrap();
//! println!("Package Architecture: {}", arch);
//!
//! let mut data = pkg.data().unwrap();
//! let dir = tempfile::TempDir::new().unwrap();
//! data.unpack(dir).unwrap();
//! ```

use std::io::Read;

mod error;
pub use error::Error;

mod control;
pub use control::Control;

mod debian_binary;
use debian_binary::{parse_debian_binary_contents, DebianBinaryVersion};

type Result<T> = std::result::Result<T, Error>;

enum ReadState {
    Opened,
    ControlRead,
    DataRead,
}

/// A debian package represented by the control data the archive holding all the
/// information
pub struct DebPkg<R: Read> {
    /// How far we've read through the debian package. This is especially
    /// important when R only implements Read and not Seek since we will not be
    /// able to Read backwards.
    state: ReadState,

    /// The major and minor fomat version of the debian package
    format_version: DebianBinaryVersion,

    /// The ar archive in which the debian package is contained
    archive: ar::Archive<R>,
}

fn validate_debian_binary<'a, R: 'a + Read>(
    entry: &mut ar::Entry<'a, R>,
) -> Result<DebianBinaryVersion> {
    let identifier = "debian-binary";

    if entry.header().identifier() == identifier.as_bytes() {
        parse_debian_binary_contents(entry)
    } else {
        Err(Error::MissingDebianBinary)
    }
}

impl<'a, R: 'a + Read> DebPkg<R> {
    /// Parses a debian package out of reader
    ///
    /// # Arguments
    ///
    /// * `reader` - A type which implements `std::io::Read` and `std::io::Seek`
    ///              and is formatted as an ar archive
    ///
    /// # Example
    ///
    /// ```no_run
    /// use debpkg::DebPkg;
    /// let file = std::fs::File::open("test.deb").unwrap();
    /// let pkg = DebPkg::parse(file).unwrap();
    /// ```
    pub fn parse(reader: R) -> Result<DebPkg<R>> {
        let mut archive = ar::Archive::new(reader);
        let mut debian_binary_entry = match archive.next_entry() {
            Some(Ok(entry)) => entry,
            Some(Err(err)) => return Err(Error::Io(err)),
            None => return Err(Error::MissingDebianBinary),
        };
        let format_version = validate_debian_binary(&mut debian_binary_entry)?;
        drop(debian_binary_entry);

        Ok(DebPkg {
            state: ReadState::Opened,
            format_version,
            archive,
        })
    }

    /// Returns the format version of the binary debian package
    pub fn format_version(&self) -> (u32, u32) {
        (self.format_version.major, self.format_version.minor)
    }

    /// Returns the control tar
    ///
    /// # Arguments
    ///
    /// * `self` - A `DebPkg` created by a call to `DebPkg::parse`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use debpkg::DebPkg;
    /// let file = std::fs::File::open("test.deb").unwrap();
    /// let mut pkg = DebPkg::parse(file).unwrap();
    /// let mut control_tar = pkg.control().unwrap();
    /// for file in control_tar.entries().unwrap() {
    ///     println!("{}", file.unwrap().path().unwrap().display());
    /// }
    /// ```
    ///
    pub fn control(&'a mut self) -> Result<tar::Archive<Box<dyn Read + 'a>>> {
        match self.state {
            ReadState::Opened => {
                let entry = match self.archive.next_entry() {
                    Some(entry) => entry?,
                    None => return Err(Error::MissingControlArchive),
                };

                self.state = ReadState::ControlRead;
                get_tar_from_entry(entry)
            }
            ReadState::ControlRead | ReadState::DataRead => Err(Error::ControlAlreadyRead),
        }
    }

    /// Returns the data tar
    ///
    /// Must only be called
    ///
    /// # Arguments
    ///
    /// * `self` - A `DebPkg` created by a call to `DebPkg::parse`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use debpkg::DebPkg;
    /// let file = std::fs::File::open("test.deb").unwrap();
    /// let mut pkg = DebPkg::parse(file).unwrap();
    /// let mut data_tar = pkg.data().unwrap();
    /// for file in data_tar.entries().unwrap() {
    ///     println!("{}", file.unwrap().path().unwrap().display());
    /// }
    /// ```
    ///
    pub fn data(&'a mut self) -> Result<tar::Archive<Box<dyn Read + 'a>>> {
        match self.control() {
            Ok(_) => (),
            Err(Error::ControlAlreadyRead) => (),
            Err(e) => return Err(e),
        };

        match self.state {
            ReadState::Opened => unreachable!(),
            ReadState::ControlRead => {
                let entry = match self.archive.next_entry() {
                    Some(entry) => entry?,
                    None => return Err(Error::MissingDataArchive),
                };

                self.state = ReadState::DataRead;
                get_tar_from_entry(entry)
            }
            ReadState::DataRead => Err(Error::DataAlreadyRead),
        }
    }
}

fn get_tar_from_entry<'a, R: 'a + Read>(
    entry: ar::Entry<'a, R>,
) -> Result<tar::Archive<Box<dyn Read + 'a>>> {
    let mut reader = entry.take(1024);
    let mut first_1kb = vec![];
    reader.read_to_end(&mut first_1kb)?;

    let is_tar = infer::archive::is_tar(&first_1kb);
    let is_gz = infer::archive::is_gz(&first_1kb);
    let is_xz = infer::archive::is_xz(&first_1kb);
    let is_bz2 = infer::archive::is_bz2(&first_1kb);
    let is_zst = infer::archive::is_zst(&first_1kb);

    let entry = std::io::Cursor::new(first_1kb).chain(reader.into_inner());

    if is_tar {
        let entry: Box<dyn Read> = Box::new(entry);
        Ok(tar::Archive::new(entry))
    } else if is_gz {
        #[cfg(feature = "gzip")]
        {
            let gz: Box<dyn Read> = Box::new(flate2::read::GzDecoder::new(entry));
            Ok(tar::Archive::new(gz))
        }
        #[cfg(not(feature = "gzip"))]
        {
            Err(Error::UnconfiguredFileFormat("gzip".to_string()))
        }
    } else if is_xz {
        #[cfg(feature = "xz")]
        {
            let xz: Box<dyn Read> = Box::new(xz2::read::XzDecoder::new_multi_decoder(entry));
            Ok(tar::Archive::new(xz))
        }
        #[cfg(not(feature = "xz"))]
        {
            Err(Error::UnconfiguredFileFormat("xz".to_string()))
        }
    } else if is_bz2 {
        #[cfg(feature = "bzip2")]
        {
            let bz2: Box<dyn Read> = Box::new(bzip2::read::BzDecoder::new(entry));
            Ok(tar::Archive::new(bz2))
        }
        #[cfg(not(feature = "bzip2"))]
        {
            Err(Error::UnconfiguredFileFormat("bzip2".to_string()))
        }
    } else if is_zst {
        #[cfg(feature = "zstd")]
        {
            let zstd: Box<dyn Read> = Box::new(zstd::stream::read::Decoder::new(entry)?);
            Ok(tar::Archive::new(zstd))
        }
        #[cfg(not(feature = "zstd"))]
        {
            Err(Error::UnconfiguredFileFormat("zstd".to_string()))
        }
    } else {
        Err(Error::UnknownEntryFormat)
    }
}
