# DebPkg

[![Build Status](https://mattschulte.visualstudio.com/debpkg/_apis/build/status/schultetwin1.debpkg?branchName=master)](https://mattschulte.visualstudio.com/debpkg/_build/latest?definitionId=2&branchName=master)

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
println!("Package Name: {}", pkg.name());
println!("Package Version: {}", pkg.version());
let arch = pkg.get("Architecture").unwrap();
println!("Package Architecture: {}", arch);
let dir = tempfile::TempDir::new().unwrap();
pkg.unpack(dir).unwrap();
```