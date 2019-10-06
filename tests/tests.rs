use DebPkg;
use tempfile::NamedTempFile;

#[test]
fn empty_ar_fails_parse() {
    let file = NamedTempFile::new().unwrap();
    let reader = file.reopen().unwrap();

    let _ = ar::Builder::new(&file);
    drop(file);

    let pkg_result = DebPkg::DebPkg::parse(&reader);
    assert!(pkg_result.is_err(), "Should fail to parse empty ar");
}