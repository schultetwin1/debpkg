# DebPkg

[![Actions Status](https://github.com/schultetwin1/debpkg/workflows/CI/badge.svg)](https://github.com/schultetwin1/debpkg/actions)
[![Rust Docs](https://docs.rs/debpkg/badge.svg)](https://docs.rs/debpkg/)
[![Crates.io Link](https://img.shields.io/crates/v/debpkg)](https://crates.io/crates/debpkg)

A Rust library to parse binary debian packages.

This library provides utilties to parse [binary debian
packages](https://www.debian.org/doc/manuals/debian-faq/ch-pkg_basics.en.html#s-deb-format)
abstracted over a reader. This API provides a streaming interface to avoid
loading the entire debian package into RAM.

This library only parses binary debian packages. It does not attempt to
write binary debian packages.

## Supported Debian Package Versions

This package only supports version 2.0 of debian packages. Older versions
are not currently supported.

## Examples

Parsing a debian package

```rust
let file = std::fs::File::open("test.deb").unwrap();
let mut pkg = debpkg::DebPkg::parse(file).unwrap();
let mut control_tar = pkg.control().unwrap();
let control = debpkg::Control::extract(control_tar).unwrap();
println!("Package Name: {}", control.name());
println!("Package Version: {}", control.version());
let arch = control.get("Architecture").unwrap();
println!("Package Architecture: {}", arch);

let mut data = pkg.data().unwrap();
let dir = tempfile::TempDir::new().unwrap();
data.unpack(dir).unwrap();
```
