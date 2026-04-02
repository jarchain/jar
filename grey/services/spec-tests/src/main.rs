//! Bless tool for spec test PVM blobs.
//!
//! Writes compiled service blobs to a target directory for use by
//! accumulate test vectors via the `blob_file` JSON syntax.
//!
//! Usage: `cargo run -p spec-tests -- bless <output-dir>`

include!(concat!(env!("OUT_DIR"), "/spec_blobs.rs"));

use std::{fs, path::PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("bless") => {
            let dir = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("spec/tests/vectors/accumulate/blobs"));

            bless(&dir);
        }
        _ => {
            eprintln!("Usage: spec-tests bless <output-dir>");
            eprintln!();
            eprintln!("Writes compiled PVM blobs to <output-dir>/minimal.pvm and bootstrap.pvm");
            std::process::exit(1);
        }
    }
}

fn bless(dir: &PathBuf) {
    fs::create_dir_all(dir).expect("failed to create output directory");

    let minimal_path = dir.join("minimal.pvm");
    fs::write(&minimal_path, MINIMAL_BLOB).expect("failed to write minimal.pvm");
    eprintln!(
        "wrote {} ({} bytes)",
        minimal_path.display(),
        MINIMAL_BLOB.len()
    );

    let bootstrap_path = dir.join("bootstrap.pvm");
    fs::write(&bootstrap_path, BOOTSTRAP_BLOB).expect("failed to write bootstrap.pvm");
    eprintln!(
        "wrote {} ({} bytes)",
        bootstrap_path.display(),
        BOOTSTRAP_BLOB.len()
    );
}
