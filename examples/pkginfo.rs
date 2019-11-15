extern crate debpkg;

use std::env;
use std::path::Path;
use std::fs::File;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: pkgname <path>");
        process::exit(1);
    }

    let deb_path = Path::new(&args[1]);

    if !deb_path.exists() {
        println!("\"{}\" does not exist", deb_path.display());
        process::exit(1);
    }

    let deb_file = match File::open(deb_path) {
        Ok(file) => file,
        Err(e) => {
            println!("ERROR: Failed to open debian file \"{}\"", deb_path.display());
            println!("       {}", e);
            process::exit(1);
        }
    };

    let pkg = match debpkg::DebPkg::parse(deb_file) {
        Ok(pkg) => pkg,
        Err(e) => {
            println!("ERROR: Failed to parse debian file \"{}\"", deb_path.display());
            println!("       {}", e);
            process::exit(1);
        }
    };

    let tags = pkg.control_tags();

    for tag in tags {
        println!("{}: {}", tag, pkg.get(tag).unwrap());
    }
}