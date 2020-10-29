macro_rules! fail {
    ($n:ident $($a:ident)+ = $m:literal) => {
        pub fn $n($( $a: impl std::fmt::Display ),+) -> impl FnOnce() -> String {
            move || format!($m, $( $a ),+)
        }
    };
}

fail!(connect addr = "Failed to connect to {}");
