//! Builds the practice repository (issue #117) under the folder given as the
//! first argument (default: the temp folder) and prints its path:
//! `cargo run -p michi --example make-practice -- R:\lab`

fn main() {
    let parent = std::env::args_os().nth(1).map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    if let Err(e) = std::fs::create_dir_all(&parent) {
        eprintln!("cannot create {}: {e}", parent.display());
        std::process::exit(1);
    }
    match michi_core::practice::create(&parent) {
        Ok(dir) => println!("{}", dir.display()),
        Err(e) => {
            eprintln!("practice repository failed: {e}");
            std::process::exit(1);
        }
    }
}
