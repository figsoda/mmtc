macro_rules! fail {
    ($n:ident $($a:ident)+ = $m:literal) => {
        pub fn $n($( $a: impl std::fmt::Display ),+) -> impl FnOnce() -> String {
            move || format!($m, $( $a ),+)
        }
    };
}

fail!(connect addr = "Failed to connect to {}");
fail!(parse_addr addr = "Faield to parse {} as an address");
fail!(parse_cfg path = "Failed to parse configuration file {}");
fail!(read path = "Failed to read file {}");
