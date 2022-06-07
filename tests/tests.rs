use tempfile::NamedTempFile;

use std::convert::TryFrom;
use std::env;

use assert_matches::assert_matches;

fn get_deb_path(filename: &str) -> std::path::PathBuf {
    let root_dir = &env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut source_path = std::path::PathBuf::from(root_dir);
    source_path.push("tests");
    source_path.push("debs");
    source_path.push(filename);
    source_path
}

#[test]
fn empty_ar_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let _ = ar::Builder::new(&file);
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader).err().unwrap();
    assert_matches!(pkg_result, debpkg::Error::Io(_));
}

#[test]
fn ar_with_out_debian_binary_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-trinary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader).err().unwrap();
    assert_matches!(pkg_result, debpkg::Error::MissingDebianBinary);
}

#[test]
fn ar_with_wrong_debian_binary_content_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "3.0\n".as_bytes()).unwrap();
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader).err().unwrap();
    assert_matches!(pkg_result, debpkg::Error::InvalidVersion);
}

#[test]
fn ar_with_only_debian_binary_fails_control() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();
    drop(file);

    let mut pkg = debpkg::DebPkg::parse(&reader).unwrap();
    let control_result = pkg.control().err().unwrap();
    assert_matches!(control_result, debpkg::Error::MissingControlArchive);
}

#[test]
fn ar_with_empty_control_tar_fails_control_extract() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();

    let control_tar = tar::Builder::new(std::vec::Vec::new());
    let control_tar = control_tar.into_inner().unwrap();

    let header = ar::Header::new(
        b"control.tar".to_vec(),
        u64::try_from(control_tar.len()).unwrap(),
    );
    archive.append(&header, &control_tar[..]).unwrap();
    drop(file);

    let mut pkg = debpkg::DebPkg::parse(&reader).unwrap();
    let control_result = pkg.control().err().unwrap();
    assert_matches!(control_result, debpkg::Error::UnknownEntryFormat);
}

#[test]
fn ar_with_empty_control_fails_extract() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();

    let control_file_contents = b"control";

    let mut header = tar::Header::new_ustar();
    header.set_size(u64::try_from(control_file_contents.len()).unwrap());
    header.set_cksum();

    let mut control_tar = tar::Builder::new(std::vec::Vec::new());
    control_tar
        .append_data(
            &mut header,
            std::path::Path::new("./control"),
            &control_file_contents[..],
        )
        .unwrap();

    let control_tar = control_tar.into_inner().unwrap();

    let header = ar::Header::new(
        b"control.tar".to_vec(),
        u64::try_from(control_tar.len()).unwrap(),
    );
    archive.append(&header, &control_tar[..]).unwrap();
    drop(file);

    let mut pkg = debpkg::DebPkg::parse(&reader).unwrap();
    let control_tar = pkg.control().unwrap();
    let control_result = debpkg::Control::extract(control_tar).err().unwrap();
    assert_matches!(control_result, debpkg::Error::InvalidControlFile);
}

#[test]
fn xz_utils_parses() {
    let xz_deb_path = get_deb_path("xz-utils_5.2.4-1_amd64.deb");
    let xz_deb = std::fs::File::open(xz_deb_path).unwrap();

    let mut pkg = debpkg::DebPkg::parse(xz_deb).unwrap();
    let control_tar = pkg.control().unwrap();
    let control = debpkg::Control::extract(control_tar).unwrap();
    assert!(control.name() == "xz-utils");

    let mut data = pkg.data().unwrap();

    let dir = tempfile::TempDir::new().unwrap();
    data.unpack(dir).unwrap();
    drop(data);

    let (major, minor) = pkg.format_version();
    assert!(major == 2);
    assert!(minor == 0);
}

#[test]
fn libgssglue_utils_parses() {
    let libgssglue_deb_path = get_deb_path("libgssglue1_0.3-4_amd64.deb");
    let libgssglue_deb = std::fs::File::open(libgssglue_deb_path).unwrap();

    let mut pkg = debpkg::DebPkg::parse(libgssglue_deb).unwrap();
    let control_tar = pkg.control().unwrap();
    let control = debpkg::Control::extract(control_tar).unwrap();
    assert!(control.name() == "libgssglue1");
    assert!(control.version() == "0.3-4");
    assert!(control.get("Architecture").unwrap() == "amd64");
    assert!(control.get("ARCHitecture").unwrap() == "amd64");
    assert!(control.get("BLAH").is_none());

    let mut data = pkg.data().unwrap();

    let dir = tempfile::TempDir::new().unwrap();
    data.unpack(dir).unwrap();
    drop(data);

    let (major, minor) = pkg.format_version();
    assert!(major == 2);
    assert!(minor == 0);
}
