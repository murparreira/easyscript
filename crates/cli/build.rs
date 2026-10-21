//! Embed the read view's single-file page, built by `npm run build:page`, when it exists.
//! Without it the binary still builds; `easyscript build` then says how to get it.

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/ui/dist-page/index.html");
    println!("cargo:rerun-if-changed={}", page.display());
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo")).join("page.html");
    let template = fs::read_to_string(&page).unwrap_or_default();
    fs::write(out, template).expect("OUT_DIR is writable");
}
