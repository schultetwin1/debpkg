use std::io::{BufRead, BufReader, Read};
use std::string::String;
use std::hash::{Hash, Hasher};
use std::vec::Vec;

use crate::{Error, Result};

use log::{warn};
use indexmap::IndexMap;
use regex::Regex;

// Tag is used to represent the tag of a field in a debian control file. Tag
// essentially creates a string which is case insensitive.
#[derive(Debug)]
struct Tag(String);

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        let mut x  = self.0.chars();
        let mut y = other.0.chars();
        loop {
            match (x.next(), y.next()) {
                (Some(a), Some(b)) if a.to_ascii_lowercase() == b.to_ascii_lowercase() => continue,
                (None, None) => return true,
                _ => return false
            }
        }
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state);
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Tag(s.to_owned())
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Tag(s)
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

type Paragraph = IndexMap<Tag, Vec<String>>;

/// Stores the Debian package's control information
#[derive(Debug)]
pub struct Control {
    paragraph: Paragraph,
}

impl Control {
    fn new() -> Control {
        Control {
            paragraph: Paragraph::default(),
        }
    }

    pub fn parse<R: Read>(reader: R) -> Result<Control> {
        let buf_reader = BufReader::new(reader);
        let mut lines = buf_reader.lines();

        let mut ctrl = Control::new();

        let comment_regex = Regex::new(r"^#.*$").unwrap();
        let continuation_regex = Regex::new(r"^\s+(?P<continuation>\S.*)$").unwrap();
        let paragraph_sep_regex = Regex::new(r"^\s*$").unwrap();
        let field_regex = Regex::new(r"^(?P<field_name>[\w-]+):(?P<field_value>.*)$").unwrap();

        let mut curr_name: Option<Tag> = None;

        loop {
            let line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => return Err(Error::Io(e)),
                None => break, // EOF
            };

            if comment_regex.is_match(&line) {
                continue;

            } else if paragraph_sep_regex.is_match(&line) {
                // TODO: This is technically an error but ignoring for now
                warn!("Unexpected paragraph seperation");
                continue;

            } else if let Some(captures) = field_regex.captures(&line) {
                let field_name = captures.name("field_name").unwrap().as_str().trim();
                let field_value = captures.name("field_value").unwrap().as_str().trim();

                let mut data = std::vec::Vec::default();
                data.push(field_value.to_owned());
                let field_tag: Tag = field_name.into();
                ctrl.paragraph.insert(field_tag, data);
                let field_tag: Tag = field_name.into();
                curr_name = Some(field_tag);

            } else if let Some(captures) = continuation_regex.captures(&line) {
                match curr_name {
                    Some(ref name) => {
                        let continuation = captures.name("continuation").unwrap().as_str();
                        let data = ctrl.paragraph.get_mut(name).unwrap();
                        data.push(continuation.to_owned());
                    },
                    None => return Err(Error::InvalidControlFile)
                };
            }
        }

        if !ctrl.paragraph.contains_key(&Tag::from("package")) {
            return Err(Error::MissingPackageName);
        }

        if !ctrl.paragraph.contains_key(&Tag::from("version")) {
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
        let field_name = Tag::from(field_name);
        match self.paragraph.get(&field_name) {
            Some(lines) => Some(&lines[0]),
            None => None,
        }
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.paragraph.keys().map(|i| i.as_ref() )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn empty_control_file_fails() {
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
        let ctrl = Control::parse(
            "package: name\nversion: 1.8.2\nDescription: short\n very\n long".as_bytes(),
        )
        .unwrap();
        assert!(ctrl.name() == "name");
        assert!(ctrl.version() == "1.8.2");
        // let desc = ctrl.get("description").unwrap();
        // assert!(desc[0] == "short");
        // assert!(desc[1] == "very");
        // assert!(desc[2] == "long");
    }

    #[test]
    fn control_starting_with_continuation_fails() {
        let err =
            Control::parse(" continue\npackage: name\nversion: 1.8.2".as_bytes()).unwrap_err();
        assert_matches!(err, Error::InvalidControlFile);
    }

    #[test]
    fn control_keys_list_everything() {
        let ctrl = Control::parse(
            "package: name\nversion: 1.8.2\nTest: a".as_bytes()
        )
        .unwrap();
        let tags: std::vec::Vec<&str> = ctrl.tags().collect();
        assert!(tags.len() == 3);
    }

    #[test]
    fn control_keys_captures_dash() {
        let ctrl = Control::parse(
            "package: name\nversion: 1.8.2\nInstalled-Size: a".as_bytes()
        )
        .unwrap();
        let tags: std::vec::Vec<&str> = ctrl.tags().collect();
        assert!(tags.len() == 3);
    }
}
