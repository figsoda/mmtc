use std::{
    env,
    fs::{create_dir_all, File},
    path::Path,
};

use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;

include!("src/cli.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=GEN_ARTIFACTS");

    if let Some(dir) = env::var_os("GEN_ARTIFACTS") {
        let out = &Path::new(&dir);
        create_dir_all(out).unwrap();
        let cmd = &mut Opts::command();

        Man::new(cmd.clone())
            .render(&mut File::create(out.join("mmtc.1")).unwrap())
            .unwrap();

        for shell in Shell::value_variants() {
            generate_to(*shell, cmd, "mmtc", out).unwrap();
        }
    }
}
