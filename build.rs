use clap::IntoApp;
use clap_generate::{generate_to, generators};

use std::{env, fs::create_dir_all, path::Path};

include!("src/cli.rs");

fn main() {
    let out = &Path::new(&env::var_os("OUT_DIR").unwrap()).join("completions");
    create_dir_all(out).unwrap();
    let app = &mut Opts::into_app();

    macro_rules! generate {
        ($($g:ident),*) => {
            $(generate_to::<generators::$g, _, _>(app, "mmtc", out);)*
        }
    }

    generate![Bash, Elvish, Fish, PowerShell, Zsh];
}
