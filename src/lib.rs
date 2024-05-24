#![cfg(target_os = "android")]

pub mod ui {
    slint::include_modules!();
}

mod app_main;
mod weather;

use std::rc::Rc;
use std::cell::RefCell;
use slint::android::android_activity;
use crate::android_activity::{ PollEvent, MainEvent };

use crate::app_main::AppHandler;

#[no_mangle]
fn android_main(android_app: slint::android::AndroidApp) -> Result<(), slint::PlatformError> {
    let app_handler = Rc::new(RefCell::new(AppHandler::new()));

    // initialize android before creating main window
    slint::android::init_with_event_listener(android_app,
         {
             let app_handler = app_handler.clone();
             move |event| {
                 match event {
                     PollEvent::Main(main_event) => {
                         match main_event {
                             MainEvent::Start => {
                                 app_handler.borrow().reload();
                             },
                             MainEvent::Resume {.. } => {
                                 app_handler.borrow().reload();
                             },
                             MainEvent::SaveState { .. } => {
                                 app_handler.borrow().save();
                             },
                             _ => {}
                         }
                     },
                     _ => {}
                 }
             }
         }).unwrap();

    {
        // create main window here
        let mut app_handler = app_handler.borrow_mut();
        app_handler.initialize_ui()?;
    }

    let app_handler = app_handler.borrow();
    app_handler.run()
}
