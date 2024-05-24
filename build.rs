use std::env;

fn main() {
    env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "true");

    slint_build::compile("ui/main.slint").unwrap();
}
