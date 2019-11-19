use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read};
use std::string::String;
use std::vec::Vec;

use crate::{Error, Result};

use indexmap::{Equivalent, IndexMap};
use log::warn;

// Tag is used to represent the tag of a field in a debian control file. Tag
// essentially creates a string which is case insensitive.
#[derive(Debug)]
struct Tag(String);

// UncasedStrRef is used to be able to search a hash map of tags without
// creating a new String.
#[derive(Debug)]
struct UncasedStrRef<'a>(&'a str);

impl<'a> UncasedStrRef<'a> {
    const fn new(s: &'a str) -> Self {
        UncasedStrRef(s)
    }
}

impl<'a> PartialEq for UncasedStrRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl<'a> Eq for UncasedStrRef<'a> {}

impl<'a> Hash for UncasedStrRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for c in self.0.as_bytes() {
            c.to_ascii_lowercase().hash(state)
        }
    }
}

impl<'a> From<&'a str> for UncasedStrRef<'a> {
    fn from(s: &'a str) -> Self {
        UncasedStrRef::new(s)
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl<'a> PartialEq<UncasedStrRef<'a>> for Tag {
    fn eq(&self, other: &UncasedStrRef) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl<'a> PartialEq<Tag> for UncasedStrRef<'a> {
    fn eq(&self, other: &Tag) -> bool {
        self.0.eq_ignore_ascii_case(other.0.as_str())
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Both UncasedStrRef and Tag must hash the same way in order for to use
        // the Equivalent trait of IndexMap
        UncasedStrRef::from(self.as_ref()).hash(state)
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Tag(s.to_owned())
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> Equivalent<UncasedStrRef<'a>> for Tag {
    fn equivalent(&self, key: &UncasedStrRef) -> bool {
        self == key
    }
}

impl<'a> Equivalent<Tag> for UncasedStrRef<'a> {
    fn equivalent(&self, key: &Tag) -> bool {
        self == key
    }
}

#[derive(Debug)]
enum FieldBody {
    Simple(String),

    // Currently binary debian packages don't have any folded fields
    #[allow(dead_code)]
    Folded(String),

    Multiline(String, Vec<String>)
}

type Paragraph = IndexMap<Tag, FieldBody>;

const DESCRIPTION: UncasedStrRef = UncasedStrRef::new("Description");
const PACKAGE: UncasedStrRef = UncasedStrRef::new("Package");
const VERSION: UncasedStrRef = UncasedStrRef::new("Version");

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
                    continue;
                }

                Some(' ') | Some('\t') => {
                    // contiuation of the current field
                    match curr_name {
                        Some(ref name) => {
                            let continuation = line.trim();
                            let data = ctrl.paragraph.get_mut(name).unwrap();
                            match data {
                                FieldBody::Simple(_value) => unreachable!(),
                                FieldBody::Folded(value) => { 
                                    value.push(' ');
                                    value.push_str(continuation);
                                },
                                FieldBody::Multiline(_first, other) => {
                                    other.push(continuation.to_owned());
                                }

                            };
                        }
                        None => return Err(Error::InvalidControlFile),
                    };
                }

                Some(_) => {
                    // new field
                    let line = line.trim();
                    let mut split = line.splitn(2, ':');
                    let field_name = match split.next() {
                        Some(ref field_name) => field_name.trim(),
                        None => return Err(Error::InvalidControlFile),
                    };
                    let field_value = match split.next() {
                        Some(ref field_name) => field_name.trim(),
                        None => return Err(Error::InvalidControlFile),
                    };
                    let field_tag: Tag = field_name.into();
                    let data = if field_tag == DESCRIPTION {
                        FieldBody::Multiline(field_value.to_owned(), Vec::default())
                    } else {
                        FieldBody::Simple(field_value.to_owned())
                    };
                    if let Some(_value) = ctrl.paragraph.insert(field_tag, data) {
                        return Err(Error::InvalidControlFile);
                    }
                    let field_tag: Tag = field_name.into();
                    curr_name = Some(field_tag);
                }

                None => {
                    // Paragraph seperation
                    // TODO: This is technically an error but ignoring for now
                    warn!("Unexpected paragraph seperation");
                    continue;
                }
            }
        }

        if !ctrl.paragraph.contains_key(&PACKAGE) {
            return Err(Error::MissingPackageName);
        }

        if !ctrl.paragraph.contains_key(&VERSION) {
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
        let (_, long) = match self.paragraph.get(&DESCRIPTION)? {
            FieldBody::Simple(_) | FieldBody::Folded(_) => unreachable!(),
            FieldBody::Multiline(short, long) => (short, long)
        };
        match long.len() {
            0 => None,
            _ => Some(long.join("\n")),
        }
    }

    pub fn get(&self, field_name: &str) -> Option<&str> {
        match self.paragraph.get(&UncasedStrRef::from(field_name)) {
            Some(FieldBody::Simple(value)) | Some(FieldBody::Folded(value)) => Some(value.as_str()),
            Some(FieldBody::Multiline(value, _)) => Some(value.as_str()),
            None => None,
        }
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.paragraph.keys().map(|i| i.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn empty_control_file_fails() {
        assert!(Control::parse(&b""[..]).is_err());
    }

    #[test]
    fn only_name_fails_parse() {
        let err = Control::parse(&b"package: name_only"[..]).unwrap_err();
        assert_matches!(err, Error::MissingPackageVersion);
    }

    #[test]
    fn only_version_fails_parse() {
        let err = Control::parse(&b"version: 1.8.2"[..]).unwrap_err();
        assert_matches!(err, Error::MissingPackageName);
    }

    #[test]
    fn name_and_version_parse() {
        let ctrl = Control::parse(&b"package: name\nversion: 1.8.2"[..]).unwrap();
        assert!(ctrl.name() == "name");
        assert!(ctrl.version() == "1.8.2");
    }

    #[test]
    fn proper_description_parse() {
        let ctrl =
            Control::parse(&b"package: name\nversion: 1.8.2\nDescription: short\n very\n long"[..])
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
        let err = Control::parse(&b" continue\npackage: name\nversion: 1.8.2"[..]).unwrap_err();
        assert_matches!(err, Error::InvalidControlFile);
    }

    #[test]
    fn control_keys_list_everything() {
        let ctrl = Control::parse(&b"package: name\nversion: 1.8.2\nTest: a"[..]).unwrap();
        let tags: std::vec::Vec<&str> = ctrl.tags().collect();
        assert!(tags.len() == 3);
    }

    #[test]
    fn control_keys_captures_dash() {
        let ctrl =
            Control::parse(&b"package: name\nversion: 1.8.2\nInstalled-Size: a"[..]).unwrap();
        let tags: std::vec::Vec<&str> = ctrl.tags().collect();
        assert!(tags.len() == 3);
    }

    #[test]
    fn control_non_continuation_line_fails() {
        let err = Control::parse(&b"package: name\nthis is wrong"[..]).unwrap_err();
        assert_matches!(err, Error::InvalidControlFile);
    }

    #[test]
    fn duplicate_fields_fails_parsing() {
        let err =
            Control::parse(&b"package: name\nversion: 1.8.2\npackage: name2"[..]).unwrap_err();
        assert_matches!(err, Error::InvalidControlFile);
    }
}
