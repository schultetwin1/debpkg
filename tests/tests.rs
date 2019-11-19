use debpkg;
use tempfile::NamedTempFile;

use std::convert::TryFrom;
use std::env;

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

    let pkg_result = debpkg::DebPkg::parse(&reader);
    assert!(pkg_result.is_err(), "Should fail to parse empty ar");
}

#[test]
fn ar_with_out_debian_binary_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-trinary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader);
    assert!(
        pkg_result.is_err(),
        "Should fail to parse ar without debian-binary file"
    );
}

#[test]
fn ar_with_wrong_debian_binary_content_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "3.0\n".as_bytes()).unwrap();
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader);
    assert!(
        pkg_result.is_err(),
        "Should fail to parse ar with debian-binary file with the wrong version"
    );
}

#[test]
fn ar_with_only_debian_binary_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader);
    assert!(
        pkg_result.is_err(),
        "Should fail to parse ar with only debian binary"
    );
}

#[test]
fn ar_with_empty_control_tar_fails_parse() {
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

    let pkg_result = debpkg::DebPkg::parse(&reader);
    assert!(
        pkg_result.is_err(),
        "Should fail to parse ar with only debian binary"
    );
}

#[test]
fn ar_with_empty_control_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let mut archive = ar::Builder::new(&file);
    let header = ar::Header::new(b"debian-binary".to_vec(), 4);
    archive.append(&header, "2.0\n".as_bytes()).unwrap();

    let mut header = tar::Header::new_ustar();
    header.set_size(u64::try_from("control".len()).unwrap());
    header.set_cksum();

    let mut control_tar = tar::Builder::new(std::vec::Vec::new());
    control_tar
        .append_data(
            &mut header,
            std::path::Path::new("control"),
            &b"control"[..],
        )
        .unwrap();
    let control_tar = control_tar.into_inner().unwrap();

    let header = ar::Header::new(
        b"control.tar".to_vec(),
        u64::try_from(control_tar.len()).unwrap(),
    );
    archive.append(&header, &control_tar[..]).unwrap();
    drop(file);

    let pkg_result = debpkg::DebPkg::parse(&reader);
    assert!(
        pkg_result.is_err(),
        "Should fail to parse ar with only debian binary"
    );
}

#[test]
fn xz_utils_parses() {
    let xz_deb_path = get_deb_path("xz-utils_5.2.4-1_amd64.deb");
    let xz_deb = std::fs::File::open(xz_deb_path).unwrap();

    let mut pkg = debpkg::DebPkg::parse(xz_deb).unwrap();
    assert!(pkg.name() == "xz-utils");

    let dir = tempfile::TempDir::new().unwrap();
    pkg.unpack(dir).unwrap();
}

#[test]
fn libgssglue_utils_parses() {
    let libgssglue_deb_path = get_deb_path("libgssglue1_0.3-4_amd64.deb");
    let libgssglue_deb = std::fs::File::open(libgssglue_deb_path).unwrap();

    let mut pkg = debpkg::DebPkg::parse(libgssglue_deb).unwrap();
    assert!(pkg.name() == "libgssglue1");
    assert!(pkg.version() == "0.3-4");
    assert!(pkg.get("Architecture").unwrap() == "amd64");
    assert!(pkg.get("ARCHitecture").unwrap() == "amd64");
    assert!(pkg.get("BLAH").is_none());

    let dir = tempfile::TempDir::new().unwrap();
    pkg.unpack(dir).unwrap();
}
