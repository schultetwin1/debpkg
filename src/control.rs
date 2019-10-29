use std::io::{BufRead, BufReader, Read};
use std::string::String;
use std::vec::Vec;

use crate::{Error, Result};

use regex::Regex;

type Paragraph = std::collections::HashMap<String, Vec<String>>;

#[derive(Debug)]
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
        let field_regex = Regex::new(r"^(?P<field_name>\w+):(?P<field_value>.*)$").unwrap();

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
                if curr_name.is_empty() {
                    return Err(Error::InvalidControlFile);
                }
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
        self.get("Package").unwrap()
    }

    pub fn version(&self) -> &str {
        self.get("Version").unwrap()
    }

    pub fn get(&self, field_name: &str) -> Option<&str> {
        match self.paragraph.get(field_name.to_lowercase().as_str()) {
            Some(lines) => Some(&lines[0]),
            None => None
        }
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

    #[test]
    fn only_name_fails_parse() {
        let err = Control::parse("package: name_only".as_bytes()).unwrap_err();
        assert_matches!(err, Error::MissingPackageVersion);
    }

    #[test]
    fn only_version_fails_parse() {
        let err = Control::parse("version: 1.8.2".as_bytes()).unwrap_err();
        assert_matches!(err, Error::MissingPackageName);
    }

    #[test]
    fn name_and_version_parse() {
        let ctrl = Control::parse("package: name\nversion: 1.8.2".as_bytes()).unwrap();
        assert!(ctrl.name() == "name");
        assert!(ctrl.version() == "1.8.2");
    }

    #[test]
    fn proper_description_parse() {
        let ctrl = Control::parse("package: name\nversion: 1.8.2\nDescription: short\n very\n long".as_bytes()).unwrap();
        assert!(ctrl.name() == "name");
        assert!(ctrl.version() == "1.8.2");
        let desc = ctrl.paragraph.get("description").unwrap();
        assert!(desc[0] == "short");
        assert!(desc[1] == "very");
        assert!(desc[2] == "long");
    }

    #[test]
    fn control_starting_with_continuation_fails() {
        let err = Control::parse(" continue\npackage: name\nversion: 1.8.2".as_bytes()).unwrap_err();
        assert_matches!(err, Error::InvalidControlFile);
    }
}