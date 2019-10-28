use std::io::{BufRead, BufReader, Read};
use std::string::String;
use std::vec::Vec;

use crate::{Error, Result};

use regex::Regex;

type Paragraph = std::collections::HashMap<String, Vec<String>>;

pub struct Control {
    paragraph: Paragraph
}

impl Control {
    fn new() -> Control {
        Control { paragraph: Paragraph::default() }
    }

    pub fn parse<R: Read>(reader: R) -> Result<Control> {
        let buf_reader = BufReader::new(reader);
        let mut lines = buf_reader.lines();

        let mut ctrl = Control::new();

        let comment_regex = Regex::new(r"^#.*$").unwrap();
        let continuation_regex = Regex::new(r"^\s+(?P<continuation>\S.*)$").unwrap();
        let paragraph_sep_regex = Regex::new(r"^\s*$").unwrap();
        let field_regex = Regex::new(r"(?P<field_name>\w+):(?P<field_value>.*)$").unwrap();

        let mut curr_name: String = String::default();

        loop {
            let line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => return Err(Error::Io(e)),
                None => break // EOF
            };

            if comment_regex.is_match(&line) {
                continue;
            }

            if paragraph_sep_regex.is_match(&line) {
                // Save off paragraph
                // TODO: This is technically an error but ignoring for now
                continue;
            }

            if let Some(captures) = field_regex.captures(&line) {
                let field_name = captures.name("field_name").unwrap().as_str().trim();
                let field_value = captures.name("field_value").unwrap().as_str().trim();

                let mut data = std::vec::Vec::default();
                data.push(field_value.to_owned());
                ctrl.paragraph.insert(field_name.to_lowercase(), data);
                curr_name = field_name.to_lowercase();
            }

            if let Some(captures) = continuation_regex.captures(&line) {
                let continuation = captures.name("continuation").unwrap().as_str();
                let data = ctrl.paragraph.get_mut(&curr_name).unwrap();
                data.push(continuation.to_owned());
            }

        };

        if !ctrl.paragraph.contains_key("package") {
            return Err(Error::MissingPackageName);
        }

        if !ctrl.paragraph.contains_key("version") {
            return Err(Error::MissingPackageVersion);
        }

        Ok(ctrl)

    }

    pub fn name(&self) -> &str {
        &self.paragraph.get("package").unwrap()[0]
    }

    pub fn version(&self) -> &str {
        &self.paragraph.get("version").unwrap()[0]
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn empty_control_file_fails () {
        assert!(Control::parse("".as_bytes()).is_err());
    }
}