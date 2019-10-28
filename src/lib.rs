use std::io::{Read, Seek};

mod error;
use error::Error;

mod control;
pub use control::Control;

mod debian_binary;
use debian_binary::{DebianBinaryVersion, parse_debian_binary_contents};

pub type Result<T> = std::result::Result<T, Error>;

pub struct DebPkg<R: Seek + Read> {
    archive: ar::Archive<R>,
    control: Control
}

fn extract_debian_binary<R: Read + Seek>(archive: &mut ar::Archive<R>) -> Result<DebianBinaryVersion> {
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
    let control_entry = entries.filter_map(|x| x.ok()).filter(|entry| entry.path().is_ok()).find(|entry| {
        let path = entry.path().unwrap();
        path == std::path::Path::new("./control")
    });
    match control_entry {
        Some(control) => {
            Control::parse(control)
        },
        None => Err(Error::MissingControlFile)
    }
}

fn extract_control_data<R: Read>(archive: &mut ar::Archive<R>) -> Result<Control> {
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
                        let reader = xz2::read::XzDecoder::new_multi_decoder(entry);
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

impl<'a, R: Read + Seek> DebPkg<R> {
    pub fn parse(reader: R) -> Result<DebPkg<R>> {
        let mut archive = ar::Archive::new(reader);

        extract_debian_binary(&mut archive)?;
        let control = extract_control_data(&mut archive)?;

        Ok(DebPkg {
            archive,
            control
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
                let xz = xz2::read::XzDecoder::new_multi_decoder(entry);
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

    pub fn name(&self) -> &str {
        self.control.name()
    }

    pub fn version(&self) -> &str {
        self.control.version()
    }
}
