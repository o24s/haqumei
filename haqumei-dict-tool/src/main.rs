use std::{env, path::Path};

use haqumei::MecabDictIndexCompiler;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = Path::new(&manifest_dir);
    let src_dir = manifest_dir.join("../haqumei/dictionary");
    let dict_dir = manifest_dir.join("../compiled");

    MecabDictIndexCompiler::new()
        .dict_dir(&src_dir)
        .out_dir(&dict_dir)
        .charset("utf-8")
        .run()
        .unwrap();
}
