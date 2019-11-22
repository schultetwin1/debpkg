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
//! println!("Package Name: {}", pkg.name());
//! println!("Package Version: {}", pkg.version());
//! let arch = pkg.get("Architecture").unwrap();
//! println!("Package Architecture: {}", arch);
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
pub struct DebPkg<R: Read> {
    /// The ar archive in which the debian package is contained
    archive: ar::Archive<R>,

    /// The deb-control information about the debian package
    control: Control,
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

fn extract_control_data<'a, R: 'a + Read>(entry: ar::Entry<'a, R>) -> Result<Control> {
    let mut tar = get_tar_from_entry(entry)?;
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

fn list_files_in_tar<'a, R: 'a + Read>(
    tar: &mut tar::Archive<R>,
) -> Result<Vec<std::path::PathBuf>> {
    let entries = tar.entries()?;
    let paths: Vec<std::path::PathBuf> = entries
        .map(|e| e.unwrap().path().unwrap().into_owned())
        .collect();
    Ok(paths)
}

impl<R: Read> DebPkg<R> {
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
        validate_debian_binary(&mut debian_binary_entry)?;
        drop(debian_binary_entry);

        let control_entry = match archive.next_entry() {
            Some(Ok(entry)) => entry,
            Some(Err(err)) => return Err(Error::Io(err)),
            None => return Err(Error::MissingControlArchive),
        };
        if !control_entry
            .header()
            .identifier()
            .starts_with(b"control.tar")
        {
            return Err(Error::MissingControlArchive);
        }
        let control = extract_control_data(control_entry)?;

        let data_entry = match archive.next_entry() {
            Some(Ok(entry)) => entry,
            Some(Err(err)) => return Err(Error::Io(err)),
            None => return Err(Error::MissingDataArchive),
        };
        if !data_entry.header().identifier().starts_with(b"data.tar") {
            return Err(Error::MissingDataArchive);
        }
        drop(data_entry);

        Ok(DebPkg { archive, control })
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

    /// Returns the short description (if it exists)
    pub fn short_description(&self) -> Option<&str> {
        self.control.short_description()
    }

    /// Returns the long description (if it exists)
    pub fn long_description(&self) -> Option<&str> {
        self.control.long_description()
    }
}

fn get_tar_from_entry<'a, R: 'a + Read>(
    entry: ar::Entry<'a, R>,
) -> Result<tar::Archive<Box<dyn Read + 'a>>> {
    let entry_ident = std::str::from_utf8(entry.header().identifier()).unwrap();

    if entry_ident.ends_with(".tar") {
        let entry: Box<dyn Read> = Box::new(entry);
        Ok(tar::Archive::new(entry))
    } else if entry_ident.ends_with(".tar.gz") {
        let gz: Box<dyn Read> = Box::new(flate2::read::GzDecoder::new(entry));
        Ok(tar::Archive::new(gz))
    } else if entry_ident.ends_with(".tar.xz") {
        let xz: Box<dyn Read> = Box::new(xz2::read::XzDecoder::new_multi_decoder(entry));
        Ok(tar::Archive::new(xz))
    } else if entry_ident.ends_with(".tar.lzma") {
        // waiting to find a good lzma lib
        unimplemented!();
    } else if entry_ident.ends_with(".tar.bz2") {
        let bz2: Box<dyn Read> = Box::new(bzip2::read::BzDecoder::new(entry));
        Ok(tar::Archive::new(bz2))
    } else if entry_ident.ends_with(".tar.zst") {
        // waiting to find a good zstd lib
        unimplemented!();
    } else {
        Err(Error::MissingDataArchive)
    }
}

impl<'a, R: 'a + Read + Seek> DebPkg<R> {
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
        let entry = self.archive.jump_to_entry(1)?;
        get_tar_from_entry(entry)
    }

    /// Returns the data tar
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
        let entry = self.archive.jump_to_entry(2)?;
        get_tar_from_entry(entry)
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

        let mut tar = get_tar_from_entry(entry)?;

        tar.unpack(dst)?;
        Ok(())
    }

    /// Lists the files in the debian package by extraction path
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
    /// let paths = pkg.list_files().unwrap();
    /// for path in paths {
    ///     println!("{}", path.display());
    /// }
    /// ```
    pub fn list_files(&mut self) -> Result<Vec<std::path::PathBuf>> {
        let entry = self.archive.jump_to_entry(2)?;
        let mut tar = get_tar_from_entry(entry)?;
        list_files_in_tar(&mut tar)
    }
}
