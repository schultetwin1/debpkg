use std::io::{Error, ErrorKind, Read, Result};
use std::string::String;

const DEBIAN_BINARY_IDENTIFIER: [u8; 16] = ['d' as u8, 'e' as u8, 'b' as u8, 'i' as u8, 'a' as u8, 'n' as u8, '-' as u8, 'b' as u8, 'i' as u8, 'n' as u8, 'a' as u8, 'r' as u8, 'y' as u8, ' ' as u8, ' ' as u8, ' ' as u8];

pub struct DebPkg<R: Read> {
    archive: ar::Archive<R>,
}

fn check_debian_binary_contents<R: Read>(entry: &mut ar::Entry<R>) -> Result<()> {
    let mut contents: String = String::new();
    entry.read_to_string(&mut contents)?;

    if contents == "2.0\n" {
        Ok(())
    } else {
        let msg = "debian-binary file did not contain correct version";
        Err(Error::new(ErrorKind::InvalidInput, msg))
    }
}

fn check_for_debian_binary<R: Read>(archive: &mut ar::Archive<R>) -> Result<()> {
    if let Some(entry_result) = archive.next_entry() {
        match entry_result {
            Ok(mut entry) => {
                if entry.header().identifier() == DEBIAN_BINARY_IDENTIFIER {
                    check_debian_binary_contents(&mut entry)?;
                } else {
                    let msg = "archive did not contain debian-binary file";
                    return Err(Error::new(ErrorKind::InvalidInput, msg));
                }
            },
            Err(err) => return Err(err)
        }
    } else {
        let msg = "An empty ar file in not a valid debian package";
        return Err(Error::new(ErrorKind::UnexpectedEof, msg));
    }
    Ok(())
}

impl<R: Read> DebPkg<R> {
    pub fn parse(reader: R) -> Result<DebPkg<R>> {
        let mut archive = ar::Archive::new(reader);

        check_for_debian_binary(&mut archive)?;

        Ok(DebPkg {
            archive
        })
    }
}

#[cfg(test)]
mod tests {
}
