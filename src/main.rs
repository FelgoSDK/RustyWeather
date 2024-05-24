#![cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]

use crate::app_main::AppHandler;

pub mod ui {
    slint::include_modules!();
}

mod app_main;
mod weather;

fn main() -> Result<(), slint::PlatformError> {
    let mut app_handler = AppHandler::new();
    app_handler.initialize_ui()?;

    app_handler.reload();
    let res = app_handler.run();
    app_handler.save();
    res
}
