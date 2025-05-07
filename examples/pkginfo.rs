extern crate debpkg;

use std::env;
use std::fs::File;
use std::path::Path;
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
            println!(
                "ERROR: Failed to open debian file \"{}\"",
                deb_path.display()
            );
            println!("       {e}");
            process::exit(1);
        }
    };

    let mut pkg = match debpkg::DebPkg::parse(deb_file) {
        Ok(pkg) => pkg,
        Err(e) => {
            println!(
                "ERROR: Failed to parse debian file \"{}\"",
                deb_path.display()
            );
            println!("       {e}");
            process::exit(1);
        }
    };

    let control_tar = match pkg.control() {
        Ok(tar) => tar,
        Err(e) => {
            println!(
                "ERROR: Failed to get control tar from debian file \"{}\"",
                deb_path.display()
            );
            println!("       {e}");
            process::exit(1);
        }
    };

    let control = match debpkg::Control::extract(control_tar) {
        Ok(control) => control,
        Err(e) => {
            println!(
                "ERROR: Failed to parse debian control file \"{}\"",
                deb_path.display()
            );
            println!("       {e}");
            process::exit(1);
        }
    };

    let tags = control.tags();

    for tag in tags {
        if tag.to_lowercase() == "description" {
            println!("{}: {}", tag, control.short_description().unwrap());
            let long_desc = control
                .long_description()
                .unwrap()
                .split('\n')
                .collect::<std::vec::Vec<&str>>()
                .join("\n ");
            println!(" {long_desc}");
        } else {
            println!("{}: {}", tag, control.get(tag).unwrap());
        }
    }
}
