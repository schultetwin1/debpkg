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
//! let mut pkg = debpkg::DebPkg::parse("test.deb").unwrap();
//! println("Package Name: {}", pkg.name());
//! println("Package Version: {}", pkg.version());
//! let arch = pkg.get("Architecture").unwrap();
//! println("Package Architecture: {}", arch);
//! let dir = tempfile::TempDir::new().unwrap();
//! pkg.unpack(dir).unwrap();
//! ```


use std::io::{Read, Seek};

mod error;
pub use error::Error;

mod control;
use control::Control;

mod debian_binary;
use debian_binary::{parse_debian_binary_contents, DebianBinaryVersion};

type Result<T> = std::result::Result<T, Error>;

/// A debian package represented by the control data the archive holding all the
/// information
pub struct DebPkg<R: Seek + Read> {
    /// The ar archive in which the debian package is contained
    archive: ar::Archive<R>,

    /// The deb-control information about the debian package
    control: Control,
}

fn extract_debian_binary<R: Read + Seek>(
    archive: &mut ar::Archive<R>,
) -> Result<DebianBinaryVersion> {
    let identifier = "debian-binary";

    if archive.count_entries()? == 0 {
        return Err(Error::MissingDebianBinary);
    }

    let mut entry = archive.jump_to_entry(0).unwrap();

    if entry.header().identifier() == identifier.as_bytes() {
        parse_debian_binary_contents(&mut entry)
    } else {
        Err(Error::MissingDebianBinary)
    }
}

fn untar_control_data<R: Read>(tar_reader: R) -> Result<Control> {
    let mut tar = tar::Archive::new(tar_reader);
    let entries = tar.entries()?;
    let control_entry = entries
        .filter_map(|x| x.ok())
        .filter(|entry| entry.path().is_ok())
        .find(|entry| {
            let path = entry.path().unwrap();
            path == std::path::Path::new("./control")
        });
    match control_entry {
        Some(control) => Control::parse(control),
        None => Err(Error::MissingControlFile),
    }
}

fn extract_control_data<R: Read>(archive: &mut ar::Archive<R>) -> Result<Control> {
    if let Some(entry_result) = archive.next_entry() {
        match entry_result {
            Ok(entry) => {
                let entry_ident = std::str::from_utf8(entry.header().identifier()).unwrap();

                match entry_ident {
                    "control.tar" => untar_control_data(entry),
                    "control.tar.gz" => {
                        let reader = flate2::read::GzDecoder::new(entry);
                        untar_control_data(reader)
                    }
                    "control.tar.xz" => {
                        let reader = xz2::read::XzDecoder::new_multi_decoder(entry);
                        untar_control_data(reader)
                    }
                    "control.tar.zst" => unimplemented!(),
                    _ => Err(Error::MissingControlArchive),
                }
            }
            Err(err) => Err(Error::Io(err)),
        }
    } else {
        Err(Error::MissingControlArchive)
    }
}

impl<'a, R: Read + Seek> DebPkg<R> {
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

        extract_debian_binary(&mut archive)?;
        let control = extract_control_data(&mut archive)?;

        Ok(DebPkg { archive, control })
    }

    /// Unpacks the filesystem in the debian package
    /// 
    /// # Arguments
    /// 
    /// * `self` - A `DebPkg` created by a call to `DebPkg::parse`
    /// 
    /// * `dst` - The path to extract all the files to
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// use debpkg::DebPkg;
    /// let file = std::fs::File::open("test.deb").unwrap();
    /// let dir = tempfile::TempDir::new().unwrap();
    /// let mut pkg = DebPkg::parse(file).unwrap();
    /// pkg.unpack(dir).unwrap();
    /// ```
    pub fn unpack<P: AsRef<std::path::Path>>(&mut self, dst: P) -> Result<()> {
        let entry = self.archive.jump_to_entry(2)?;
        let entry_ident = std::str::from_utf8(entry.header().identifier()).unwrap();

        match entry_ident {
            "data.tar" => {
                let mut tar = tar::Archive::new(entry);
                tar.unpack(dst)?;
                Ok(())
            }
            "data.tar.gz" => {
                let gz = flate2::read::GzDecoder::new(entry);
                let mut tar = tar::Archive::new(gz);
                tar.unpack(dst)?;
                Ok(())
            }
            "data.tar.xz" => {
                let xz = xz2::read::XzDecoder::new_multi_decoder(entry);
                let mut tar = tar::Archive::new(xz);
                tar.unpack(dst)?;
                Ok(())
            }
            "data.tar.zst" => unimplemented!(),
            _ => Err(Error::MissingDataArchive),
        }
    }

    /// Returns the package name
    pub fn name(&self) -> &str {
        self.control.name()
    }

    /// Returns the package version
    pub fn version(&self) -> &str {
        self.control.version()
    }

    /// Returns a specific tag in the control file if it exists
    pub fn get(&self, key: &str) -> Option<&str> {
        self.control.get(key)
    }

    /// Returns an iterator of all the tags in the control file
    pub fn control_tags(&self) -> impl Iterator<Item = &str> {
        self.control.tags()
    }
}
