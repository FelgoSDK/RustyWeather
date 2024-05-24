use serde::{Serialize, Deserialize};
use slint::Weak;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use openweather_sdk::{Language, OpenWeather, Units};
use openweather_sdk::responses::{GeocodingResponse, OneCallResponse};
use crate::weather::weatherdisplaycontroller::WeatherDisplayController;

use crate::ui;
use ui::AppWindow;

#[cfg(target_os = "android")]
use std::env;

const CITIES_STORED_FILE_NAME: &str = "cities_data.json";
const ORGANIZATION_QUALIFIER: &str = "com"; // have to match android app name in cargo.toml
const ORGANIZATION_NAME: &str = "felgo.demos"; // have to match android app name in cargo.toml
const APPLICATION_NAME: &str = "rustyweather";   // have to match app android name in cargo.toml

fn project_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        match env::var("ANDROID_DATA") {
            Ok(data_root) => {
                if data_root.is_empty() {
                    return None;
                }
                else {
                    let project_name = format!("{}.{}.{}", ORGANIZATION_QUALIFIER, ORGANIZATION_NAME, APPLICATION_NAME);
                    return Some(PathBuf::from(format!("{}/data/{}/files", data_root, project_name)));
                }
            }
            Err(_e) => {
                log::warn!("Cannot read ANDROID_DATA, persistence not avaialble.");
                return None;
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        if let Some(project_dir) = directories::ProjectDirs::from(ORGANIZATION_QUALIFIER, ORGANIZATION_NAME, APPLICATION_NAME) {
            return Some(project_dir.data_dir().to_path_buf());
        };

        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CityData {
    pub lat: f64,
    pub lon: f64,
    pub city_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeatherClient {
    pub city_data: CityData,
    pub weather_data: Option<OneCallResponse>,
    pub last_update_timestamp: i64
}

pub struct WeatherController {
    tokio_runtime: tokio::runtime::Runtime,
    weather_api: OpenWeather,
    city_clients: Arc<Mutex<Vec<WeatherClient>>>
}

// TODO: make the logic controller fully UI agnostic
impl WeatherController {
    pub fn new() -> Self {
        let api_key = std::env!("RUSTY_WEATHER_API_KEY", "The OpenWeather API must be provided!");
        let mut weather_api = OpenWeather::new(
            api_key.to_string(),
            Units::Metric,
            Language::English
        );
        weather_api.one_call.fields.minutely = false;
        weather_api.one_call.fields.hourly = false;
        weather_api.one_call.fields.alerts = false;

        let mut city_clients = vec![];
        if let Some(project_dir) = project_data_dir() {
            let cache_file = project_dir.as_path().join(CITIES_STORED_FILE_NAME);
            match WeatherController::load(cache_file) {
                Ok(loaded_data) => {
                    city_clients = loaded_data;
                    let city_count = city_clients.len();
                    log::debug!("Successfully loaded {city_count} cities");
                }
                Err(e) => log::error!("Error while loading: {e}")
            }
        } else {
            log::error!("Failed to initialize project dir. Persistent data will not be loaded");
        }

        Self {
            tokio_runtime: tokio::runtime::Runtime::new().unwrap(),
            weather_api: weather_api,
            city_clients: Arc::new(Mutex::new(city_clients)),
        }
    }

    fn load<P: AsRef<Path>>(path: P) -> Result<Vec<WeatherClient>, Box<dyn std::error::Error>> {
        log::debug!("Starting to load clients");
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let data: Vec<WeatherClient> = serde_json::from_reader(reader)?;
        Ok(data)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Saving persistent data");

        if let Some(project_dir) = project_data_dir() {
            let project_dir_path = project_dir.as_path();
            fs::create_dir_all(project_dir_path)?;
            let cache_file = project_dir_path.join(CITIES_STORED_FILE_NAME);
            log::debug!("Saving data to: {cache_file:?}");
            let file = File::create(cache_file)?;
            let mut writer = BufWriter::new(file);
            let data_clone = self.city_clients.clone();
            self.tokio_runtime.spawn(async move {
                let data = data_clone.lock().await;
                let _ = serde_json::to_writer(&mut writer, data.deref()).unwrap();
                let _ = writer.flush().unwrap();
            });
        } else {
            log::error!("Failed to initialize project_dir");
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Project dir not found")));
        }
        Ok(())
    }

    pub fn reorder_cities(&self, index: usize, new_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let city_clients = self.city_clients.clone();
        self.tokio_runtime.block_on(async move {
            let mut city_clients = city_clients.lock().await;
            city_clients.swap(index, new_index);
        });
        Ok(())
    }

    pub fn remove_city(&self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let city_clients = self.city_clients.clone();
        self.tokio_runtime.block_on(async move {
            let mut city_clients = city_clients.lock().await;
            city_clients.remove(index);
        });
        Ok(())
    }

    pub fn refresh_all(&self, weak_window: Weak<AppWindow>) {
        log::debug!("Refreshing all the clients!");

        let city_clients_clone = self.city_clients.clone();
        let weather_api = self.weather_api.clone();
        self.tokio_runtime.spawn(async move {
            let mut city_clients = city_clients_clone.lock().await;
            for client in city_clients.iter_mut() {
                client.refresh_weather(&weather_api).await;
                // TODO: Maybe would be better to add to a list and wait after.
            }

            WeatherController::update_display(weak_window, &city_clients);
            log::debug!("Refreshing weather finished!");
        });
    }

    fn update_display(weak_window: Weak<AppWindow>, city_clients: &Vec<WeatherClient>) {
            let city_data = city_clients.iter().map(
                |client| (*client).clone()).collect();
            WeatherDisplayController::update_displayed_cities_weather(weak_window, city_data);
    }

    pub fn search_location(&self, weak_window: Weak<AppWindow>, query: slint::SharedString) {
        log::debug!("Searching for: {query}");
        let weather_api = self.weather_api.clone();

        if query.is_empty() {
            WeatherDisplayController::update_location_search_results(weak_window, vec![]);
            return
        }

        self.tokio_runtime.spawn(async move {
            let res = weather_api.geocoding.get_geocoding(&query, None, None, 0).await;

            match res {
                Ok(response_data) => {
                    log::debug!("Search result: {response_data:?}");

                    let mut unique_response_data :Vec<GeocodingResponse> = Vec::new();
                    for element in response_data {
                        if !unique_response_data.iter().any(|existing_element| {
                            if existing_element.name == element.name &&
                                existing_element.country == element.country &&
                                element.state.is_some() && existing_element.state == element.state {
                                return true;
                            }
                            false
                        }) {
                            unique_response_data.push(element);
                        }
                    }

                    WeatherDisplayController::update_location_search_results(weak_window, unique_response_data);
                },
                Err(e) => {
                    log::error!("Error while searching for location: {e}");
                }
            }
        });
    }

    pub fn add_new_city(&self, weak_window: Weak<AppWindow>, city: CityData) {
        log::debug!("Adding new city: {city:?}");
        let city_clients_clone = self.city_clients.clone();
        let weather_api = self.weather_api.clone();
        self.tokio_runtime.spawn(async move {
            let mut city_clients = city_clients_clone.lock().await;
            match city_clients.iter().position(|client| client.city_data == city) {
                Some(_) => {
                    log::info!("City already present in list!");
                    WeatherDisplayController::hide_busy_screen(weak_window);
                },
                None => {
                    // Add to list and refresh
                    let mut client = WeatherClient::new(city.lat, city.lon, &city.city_name);
                    client.refresh_weather(&weather_api).await;
                    city_clients.push(client);
                    WeatherController::update_display(weak_window, &city_clients);
                }
            }
        });
    }
}

impl WeatherClient {
    pub fn new(lat: f64, lon: f64, cname: &str) -> Self {
        Self {
            city_data: CityData {
                lat: lat,
                lon: lon,
                city_name: cname.to_string(),
            },
            weather_data: None,
            last_update_timestamp: 0
        }
    }

    pub async fn refresh_weather(&mut self, weather_api: &OpenWeather) {
        let res = weather_api.one_call.call(self.city_data.lat, self.city_data.lon).await;
        log::debug!("Weather response: {res:?}");
        match res {
            Ok(response_data) => {
                self.weather_data = Some(response_data);
                self.last_update_timestamp = chrono::offset::Local::now().timestamp();
                log::info!("Response received at: {:?}", self.last_update_timestamp)
            },
            Err(e) => {
                log::error!("Error while refreshing weather: {e}");
            }
        }
    }
}
