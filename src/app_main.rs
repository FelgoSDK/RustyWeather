use std::sync::Arc;

use crate::ui::*;

use crate::weather;
use weather::WeatherController;
use weather::WeatherDisplayController;

pub struct AppHandler {
    weather_controller: Arc<WeatherController>,
    window: Option<AppWindow>,
}

impl AppHandler {
    pub fn new() -> Self {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

        Self {
            weather_controller: Arc::new(WeatherController::new()),
            window: None,
        }
    }

    pub fn save(&self) {
        log::debug!("Saving state");
        if let Err(e) = self.weather_controller.save() {
            log::warn!("Error while saving state: {}", e)
        }
    }

    pub fn reload(&self) {
        log::debug!("Reloading state");
        if let Some(window) = &self.window {
            window.global::<BusyLayerController>().invoke_set_busy();
            self.weather_controller.refresh_all(window.as_weak()); // load new weather data
        }
        else {
            log::warn!("Cannot reload state, window not available.");
        }
    }

    pub fn initialize_ui(&mut self) -> Result<(), slint::PlatformError> {
        let window = AppWindow::new().expect("Cannot create main window!");
        self.window = Some(window);

        Ok(())
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        let window = self.window.as_ref().expect("Cannot access main window!");

        let _weather_display_controller =
            WeatherDisplayController::new(&window, self.weather_controller.clone());
        window.run()
    }
}