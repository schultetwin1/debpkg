use std::io::{BufRead, BufReader, Read};
use std::string::String;
use std::hash::{Hash, Hasher};
use std::vec::Vec;

use crate::{Error, Result};

use log::{warn};
use indexmap::IndexMap;

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
        let lines = buf_reader.lines();

        let mut ctrl = Control::new();

        let mut curr_name: Option<Tag> = None;

        for line in lines {
            let line = line?;

            match line.trim_end().chars().nth(0) {
                Some('#') => {
                    // Comment line, ignore
                    continue
                },

                Some(' ') | Some('\t') => {
                    // contiuation of the field
                    match curr_name {
                        Some(ref name) => {
                            let continuation = line.trim();
                            let data = ctrl.paragraph.get_mut(name).unwrap();
                            data.push(continuation.to_owned());
                        },
                        None => return Err(Error::InvalidControlFile)
                    };
                },

                Some(_) => {
                    let line = line.trim();
                    let mut split = line.splitn(2, ':');
                    let field_name = match split.next() {
                        Some(ref field_name) => field_name.trim(),
                        None => return Err(Error::InvalidControlFile)
                    };
                    let field_value = match split.next() {
                        Some(ref field_name) => field_name.trim(),
                        None => return Err(Error::InvalidControlFile)
                    };
                    let mut data = std::vec::Vec::default();
                    data.push(field_value.to_owned());
                    let field_tag: Tag = field_name.into();
                    ctrl.paragraph.insert(field_tag, data);
                    let field_tag: Tag = field_name.into();
                    curr_name = Some(field_tag);
                },

                None => {
                    // Paragraph seperation
                    // TODO: This is technically an error but ignoring for now
                    warn!("Unexpected paragraph seperation");
                    continue;
                }
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

    pub fn short_description(&self) -> Option<&str> {
        self.get("Description")
    }

    pub fn long_description(&self) -> Option<String> {
        let desc = self.paragraph.get(&Tag::from("Description"))?;
        match desc.len() {
            0 | 1 => None,
            _ => Some(desc[1..].join("\n"))
        }
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
        let desc = ctrl.short_description().unwrap();
        assert!(desc == "short");
        let desc = ctrl.long_description().unwrap();
        assert!(desc == "very\nlong");
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

    #[test]
    fn control_non_continuation_line_fails() {
        let err = Control::parse(
            "package: name\nthis is wrong".as_bytes()
        )
        .unwrap_err();
        assert_matches!(err, Error::InvalidControlFile);
    }
}
