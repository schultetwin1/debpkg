use std::io::{BufRead, BufReader, Read};
use std::string::String;
use std::convert::TryFrom;

use crate::{Error, Result};

pub enum DebType {
    Deb,
    UDeb
}

enum ControlField {
    Package,
    PackageType,
    Version,
    Maintainer,
    Description,
    Section,
    Priority,
    InstalledSize,
    Essential,
    BuildEssential,
    Architecture,
    Origin,
    Bugs,
}

impl Into<&str> for ControlField {
    fn into(self) -> &'static str {
        match self {
            ControlField::Package => "Package",
            ControlField::PackageType => "Package-Type",
            ControlField::Version => "Version",
            ControlField::Maintainer => "Maintainer",
            ControlField::Description => "Description",
            ControlField::Section => "Section",
            ControlField::Priority => "Priority",
            ControlField::InstalledSize => "Installed-Size",
            ControlField::Essential => "Essential",
            ControlField::BuildEssential => "Build-Essential",
            ControlField::Architecture => "Architecture",
            ControlField::Origin => "Origin",
            ControlField::Bugs => "Bugs"
        }
    }
}

impl TryFrom<&str> for ControlField {
    type Error = crate::Error;

    fn try_from(string: &str) -> Result<ControlField> {
        let string_lowercase = string.to_lowercase();

        match string_lowercase.as_str() {
            "package" => Ok(ControlField::Package),
            "package-type" => Ok(ControlField::PackageType),
            "version" => Ok(ControlField::Version),
            "maintainer" => Ok(ControlField::Maintainer),
            "description" => Ok(ControlField::Description),
            "section" => Ok(ControlField::Section),
            "priority" => Ok(ControlField::Priority),
            "installed-size" => Ok(ControlField::InstalledSize),
            "essential" => Ok(ControlField::Essential),
            "build-essential" => Ok(ControlField::BuildEssential),
            "architecture" => Ok(ControlField::Architecture),
            "origin" => Ok(ControlField::Origin),
            "bugs" => Ok(ControlField::Bugs),
            _ => Err(Error::UnknownControlField)
        }
    }
}

impl TryFrom<&str> for DebType {
    type Error = crate::Error;

    fn try_from(string: &str) -> Result<DebType> {
        match string.to_lowercase().as_str() {
            "deb" => Ok(DebType::Deb),
            "udeb" => Ok(DebType::UDeb),
            _ => Err(Error::InvalidPackageType)
        }
    }
}

impl Into<&str> for DebType {
    fn into(self) -> &'static str {
        match self {
            DebType::Deb => "deb",
            DebType::UDeb => "udeb"
        }
    }
}

pub struct DebControl {
    name: String,
    pkgtype: DebType,
    version: String,
    maintainer: String,
    arch: String,
}

impl DebControl {
    pub fn parse<R: Read>(reader: R) -> Result<DebControl> {
        let buf_reader = BufReader::new(reader);

        let mut ctrl = DebControl {
            name: String::default(),
            pkgtype: DebType::Deb,
            version: String::default(),
            maintainer: String::default(),
            arch: String::default()
        };

        for line in buf_reader.lines() {
            let line = line.unwrap();

            let split: std::vec::Vec<&str> = line.splitn(2, ":").collect();
            if split.len() == 2 {
                let field_tag = split[0].to_lowercase();
                let field_text = split[1];
                let field_tag = match ControlField::try_from(field_tag.as_str()) {
                    Ok(field_tag) => field_tag,
                    Err(_e) => continue
                };

                match field_tag {
                    ControlField::Package => ctrl.set_name(field_text),
                    ControlField::PackageType => ctrl.set_package_type(field_text)?,
                    ControlField::Version => ctrl.set_version(field_text),
                    ControlField::Maintainer => ctrl.set_maintainer(field_text),
                    ControlField::Architecture => ctrl.set_arch(field_text),
                    _ => ()
                };
            }
        }
        Ok(ctrl)
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    fn set_name(&mut self, name: &str) {
        self.name.insert_str(0, name);
    }

    fn set_package_type(&mut self, package_type: &str) -> Result<()> {
        let pkgtype = DebType::try_from(package_type)?;
        self.pkgtype = pkgtype;
        Ok(())
    }

    fn set_version(&mut self, version: &str) {
        self.version.insert_str(0, version);
    }

    fn set_maintainer(&mut self, maintainer: &str) {
        self.maintainer.insert_str(0, maintainer);
    }

    fn set_arch(&mut self, arch: &str) {
        self.arch.insert_str(0, arch);
    }
}