use std::io::Read;

use crate::{Error, Result};

#[derive(Debug)]
pub struct DebianBinaryVersion {
    pub major: u32,
    pub minor: u32,
}

pub fn parse_debian_binary_contents<R: Read>(stream: &mut R) -> Result<DebianBinaryVersion> {
    let mut first_two_bytes: [u8; 2] = [0, 0];
    stream.read_exact(&mut first_two_bytes)?;

    if &first_two_bytes != b"2." {
        return Err(Error::InvalidVersion);
    }

    // note: This limits the largest minor version to 99999. Hopefully we never get above that.
    let mut string = String::new();
    for byte in stream.bytes() {
        let byte = byte?;
        if byte == b'\n' {
            break;
        }
        if !(byte as char).is_ascii_digit() {
            return Err(Error::InvalidVersion);
        }
        string.push(byte as char);
    }
    let minor = match string.as_str().parse::<u32>() {
        Ok(num) => num,
        Err(_e) => return Err(Error::InvalidVersion),
    };

    Ok(DebianBinaryVersion { major: 2, minor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn proper_debian_binary_contents_succeeds() {
        let contents = b"2.0\n";
        assert!(parse_debian_binary_contents(&mut contents.as_ref()).is_ok());
        let version = parse_debian_binary_contents(&mut contents.as_ref()).unwrap();
        assert!(version.major == 2);
        assert!(version.minor == 0);
    }

    #[test]
    fn old_version_debian_binary_contents_fails() {
        let contents = b"1.0\n";
        let result = parse_debian_binary_contents(&mut contents.as_ref());
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Error::InvalidVersion);
    }

    #[test]
    fn new_version_debian_binary_contents_fails() {
        let contents = b"3.0\n";
        let result = parse_debian_binary_contents(&mut contents.as_ref());
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Error::InvalidVersion);
    }

    #[test]
    fn empty_debian_binary_contents_fails() {
        let contents = b"";
        let result = parse_debian_binary_contents(&mut contents.as_ref());
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Error::Io(_));
    }

    #[test]
    fn windows_line_ending_debian_binary_contents_fails() {
        let contents = b"2.0\r\n";
        let result = parse_debian_binary_contents(&mut contents.as_ref());
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Error::InvalidVersion);
    }

    #[test]
    fn extra_characters_after_newline_debian_binary_contents_succeeds() {
        let contents = b"2.0\n\r";
        assert!(parse_debian_binary_contents(&mut contents.as_ref()).is_ok());
        let version = parse_debian_binary_contents(&mut contents.as_ref()).unwrap();
        assert!(version.major == 2);
        assert!(version.minor == 0);
    }

    #[test]
    fn extra_newlines_debian_binary_contents_succeeds() {
        let contents = b"2.0\n\n";
        assert!(parse_debian_binary_contents(&mut contents.as_ref()).is_ok());
        let version = parse_debian_binary_contents(&mut contents.as_ref()).unwrap();
        assert!(version.major == 2);
        assert!(version.minor == 0);
    }

    #[test]
    fn bump_minor_version_debian_binary_contents_succeeds() {
        let contents = b"2.1\n";
        assert!(parse_debian_binary_contents(&mut contents.as_ref()).is_ok());
        let version = parse_debian_binary_contents(&mut contents.as_ref()).unwrap();
        assert!(version.major == 2);
        assert!(version.minor == 1);
    }

    #[test]
    fn large_minor_version_debian_binary_contents_succeeds() {
        let contents = b"2.100\n";
        assert!(parse_debian_binary_contents(&mut contents.as_ref()).is_ok());
        let version = parse_debian_binary_contents(&mut contents.as_ref()).unwrap();
        assert!(version.major == 2);
        assert!(version.minor == 100);
    }

    #[test]
    fn new_version_new_line_debian_binary_contents_succeeds() {
        let contents = b"2.100\nTest\n";
        assert!(parse_debian_binary_contents(&mut contents.as_ref()).is_ok());
        let version = parse_debian_binary_contents(&mut contents.as_ref()).unwrap();
        assert!(version.major == 2);
        assert!(version.minor == 100);
    }
}
