macro_rules! fail {
    ($n:ident $($a:ident)+ = $m:literal) => {
        pub fn $n($( $a: impl std::fmt::Display ),+) -> impl FnOnce() -> String {
            move || format!($m, $( $a ),+)
        }
    };
}

fail!(parse_cfg path = "Failed to parse configuration file {}");
fail!(read path = "Failed to read file {}");
