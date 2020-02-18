/// Used for printing symbolic constants like GL_OUT_OF_MEMORY
macro_rules! stringify_match {
    ($on:expr, ($($id:ident),+)) => {
        match $on {
            $($id => {stringify!($id)}),+
            _ => "UNKNOWN",
        }
    };
}
