use anyhow::*;
use fs_extra::{copy_items, dir::CopyOptions};
use std::env;

fn main() -> Result<()> {
    //// Instruct cargo to re-run this script if something changes in the res/ folder.
    //println!("cargo:rerun-if-changed=res/*");

    //// OUT_DIR is an environment variable that cargo creates to specify what folder the application
    //// will be built in.
    //let out_dir = env::var("OUT_DIR")?;
    //let mut copy_options = CopyOptions::new();
    //// Overwrite files if they already exist
    //copy_options.overwrite = true;
    //let paths_to_copy = vec!["res/"];
    ////let mut paths_to_copy = Vec::new();
    ////paths_to_copy.push("res/");
    //copy_items(&paths_to_copy, out_dir, &copy_options)?;
    Ok(())
}
