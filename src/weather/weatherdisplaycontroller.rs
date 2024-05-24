use std::rc::Rc;
use std::sync::Arc;
use chrono::DateTime;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Weak};
use openweather_sdk::responses::{GeocodingResponse, OneCallResponse};

use crate::ui;
use ui::{AppWindow, BusyLayerController, CityWeather, CityWeatherInfo, WeatherForecastInfo, WeatherInfo, TemperatureInfo, GeoLocation, GeoLocationEntry};

use crate::weather::weathercontroller::{WeatherController, WeatherClient, CityData};
use crate::weather::utils::*;

pub struct WeatherDisplayController {}

fn forecast_graph_command(model: ModelRc<WeatherForecastInfo>, days_count: i32, width: f32, height: f32) -> SharedString {
    if days_count == 0 || width == 0.0 || height == 0.0 {
        return SharedString::new();
    }

    let temperatures: Vec<f32> = model.clone()
        .iter().take(days_count as usize)
        .map(|info| info.weather_info.detailed_temp.day)
        .collect();

    const MIN_MAX_MARGIN: f32 = 5.0;
    let min_temperature = match temperatures.iter().min_by(|a,b| a.total_cmp(&b)) {
        Some(min) => min - MIN_MAX_MARGIN,
        None => 0.0
    };
    let max_temperature = match temperatures.iter().max_by(|a,b| a.total_cmp(&b)) {
        Some(max) => max + MIN_MAX_MARGIN,
        None => 50.0
    };

    let max_temperature_value = max_temperature - min_temperature;
    let temperature_ratio = height / max_temperature_value;

    let day_width = width / days_count as f32;
    let max_day_shift = days_count as f32 * day_width;

    let border_command = format!(
        "M 0 0 M {max_width} 0 M {max_width} {max_temperature_value} M 0 {max_temperature_value} ",
        max_width=max_day_shift, max_temperature_value=max_temperature_value * temperature_ratio);

    let mut command = border_command;

    let day_shift = |index: f32| -> f32 {
        index * day_width + 0.5 * day_width
    };
    let day_temperature = |temperature: f32| -> f32 {
        (max_temperature - temperature) * temperature_ratio
    };

    for (index, &temperature) in temperatures.iter().enumerate() {
        if index == 0 {
            command += format!("M {x} {y} ",
                               x = day_shift(index as f32),
                               y = day_temperature(temperature)).as_str();
        }

        if let Some(next_temperature) = temperatures.get(index + 1) {
            let next_temperature = next_temperature.clone();

            let day1 = day_shift(index as f32);
            let day2 = day_shift(index as f32 + 1.0);
            let temp1 = day_temperature(temperature);
            let temp2 = day_temperature(next_temperature);

            let day_mid = (day1 + day2) / 2.0;
            let temp_mid = (temp1 + temp2) / 2.0;

            let cp_day1 = (day_mid + day1) / 2.0;
            let cp_day2 = (day_mid + day2) / 2.0;

            command += format!("Q {x1} {y1} {cx1} {cy1} Q {x2} {y2} {cx2} {cy2} ",
                               x1 = cp_day1,
                               y1 = temp1,
                               cx1 = day_mid,
                               cy1 = temp_mid,
                               x2 = cp_day2,
                               y2 = temp2,
                               cx2 = day2,
                               cy2 = temp2).as_str();
        }
    }

    SharedString::from(command)
}

impl WeatherDisplayController {
    pub fn new(window: &AppWindow, data_controller: Arc<WeatherController>) -> Self {
        window.on_refresh_all({
            let window_weak = window.as_weak();
            let weather_controller = data_controller.clone();
            move || {
                Self::refresh_all(window_weak.clone(), &weather_controller);
            }
        });

        let city_weather = window.global::<CityWeather>();
        city_weather.on_get_forecast_graph_command(forecast_graph_command);

        city_weather.on_reorder({
            let window_weak = window.as_weak();
            let weather_controller = data_controller.clone();
            move |index, new_index| {
                let window = window_weak.upgrade().unwrap();
                if let Err(e) = Self::reorder_cities(&window, &weather_controller, index, new_index) {
                    log::warn!("Failed to reorder city from {} to {}: {}", index, new_index, e);
                }
            }
        });

        city_weather.on_delete({
            let window_weak = window.as_weak();
            let weather_controller = data_controller.clone();
            move |index| {
                let window = window_weak.upgrade().unwrap();
                if let Err(e) = Self::remove_city(&window, &weather_controller, index) {
                    log::warn!("Failed to remove city from {}: {}", index, e);
                }
            }
        });

        let geo_location = window.global::<GeoLocation>();
        geo_location.on_search_location({
            let window_weak = window.as_weak();
            let weather_controller = data_controller.clone();
            move |location| {
                weather_controller.search_location(window_weak.clone(), location);
            }
        });

        geo_location.on_add_location({
            let window_weak = window.as_weak();
            let weather_controller = data_controller.clone();
            move |location| {
                let city = CityData{
                    lat: location.lat as f64, lon: location.lon as f64, city_name: String::from(&location.name)
                };
                weather_controller.add_new_city(window_weak.clone(), city);
            }
        });

        Self {}
    }

    fn refresh_all(window_weak: Weak<AppWindow>, weather_controller: &Arc<WeatherController>) {
        weather_controller.refresh_all(window_weak);
    }

    fn reorder_cities(window: &AppWindow, weather_controller: &Arc<WeatherController>, index: i32, new_index: i32) -> Result<(), Box<dyn std::error::Error>> {
        let pos : usize = index.try_into()?;
        let new_pos : usize = new_index.try_into()?;

        weather_controller.reorder_cities(pos, new_pos)?;

        let city_weather = window.global::<CityWeather>();
        let city_weather_list = city_weather.get_city_weather();

        let pos_data = city_weather_list.row_data(pos).ok_or(
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Index out of bounds")))?;
        let new_pos_data = city_weather_list.row_data(new_pos).ok_or(
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Index out of bounds")))?;

        city_weather_list.set_row_data(pos, new_pos_data);
        city_weather_list.set_row_data(new_pos, pos_data);
        Ok(())
    }

    fn remove_city(window: &AppWindow, weather_controller: &Arc<WeatherController>, index: i32) -> Result<(), Box<dyn std::error::Error>> {
        let pos : usize = index.try_into()?;

        weather_controller.remove_city(pos)?;

        let city_weather = window.global::<CityWeather>();
        let city_weather_list = city_weather.get_city_weather();

        let model = city_weather_list.as_any().downcast_ref::<slint::VecModel<CityWeatherInfo>>()
            .expect("CityWeatherInfo model is not provided!");

        model.remove(pos);
        Ok(())
    }

    pub fn hide_busy_screen(weak_window: Weak<AppWindow>) {
        weak_window.upgrade_in_event_loop(move |window| {
            window.global::<BusyLayerController>().invoke_unset_busy();
        }).unwrap();
    }

    pub fn update_displayed_cities_weather(weak_window: Weak<AppWindow>, data: Vec<WeatherClient>) {
        weak_window.upgrade_in_event_loop(move |window| {
            let display_vector: Vec<CityWeatherInfo> =
                data.iter().map(|city_client| Self::city_weather_info_from_client(&city_client)).collect();
            window.global::<CityWeather>().set_city_weather(
                Rc::new(slint::VecModel::from(display_vector)).into()
            );

            window.global::<BusyLayerController>().invoke_unset_busy();
        }).unwrap();
    }

    pub fn update_location_search_results(weak_window: Weak<AppWindow>, result: Vec<GeocodingResponse>) {
        weak_window.upgrade_in_event_loop(move |window| {
            let display_vector: Vec<GeoLocationEntry> =
                result.iter().map(|element| GeoLocationEntry {
                    name: SharedString::from(&element.name),
                    state: SharedString::from(element.state.as_deref().unwrap_or_default()),
                    country: SharedString::from(&element.country),
                    lat: element.lat as f32,
                    lon: element.lon as f32,
                }).collect();
            window.global::<GeoLocation>().set_result_list(
                Rc::new(slint::VecModel::from(display_vector)).into()
            );
        }).unwrap();
    }

    fn current_weather_info_from_response(weather_response: &Option<OneCallResponse>) -> WeatherInfo {
        if let Some(weather_data) = weather_response {
            if let Some(current) = &weather_data.current {
                let weather_details = &current.weather[0];
                let today_weather_info = weather_data.daily.as_ref().and_then(|daily| daily.first());

                let detailed_temp = match today_weather_info {
                    Some(info) => {
                        let temp = info.temp;
                        TemperatureInfo {
                            min: temp.min as f32,
                            max: temp.max as f32,

                            morning: temp.morn as f32,
                            day: temp.day as f32,
                            evening: temp.eve as f32,
                            night: temp.night as f32,
                        }
                    }
                    None => {
                        TemperatureInfo {
                            min: current.temp as f32,
                            max: current.temp as f32,

                            morning: current.temp as f32,
                            day: current.temp as f32,
                            evening: current.temp as f32,
                            night: current.temp as f32,
                        }
                    }
                };

                return WeatherInfo {
                    description: SharedString::from(&weather_details.description),
                    icon_source: SharedString::from(&weather_details.icon),
                    current_temp: current.temp as f32,
                    detailed_temp: detailed_temp,
                    uv: 0 as i32,
                    precipitation_prob: 0 as f32,
                    rain: 0 as f32,
                    snow: 0 as f32,
                }
            }
        }

        WeatherInfo::default()
    }

    fn forecast_weather_info_from_response(weather_response: &Option<OneCallResponse>) -> Vec<WeatherForecastInfo> {
        let mut forecast_weather_info:Vec<WeatherForecastInfo> = vec![];

        if let Some(weather_data) = weather_response {
            if let Some(daily_weather_data) = &weather_data.daily {

                let mut iter = daily_weather_data.into_iter();
                while let Some(day_weather_data) = iter.next() {
                    if let Some(datetime) = DateTime::from_timestamp(day_weather_data.datetime as i64, 0) {
                        let weather_details = &day_weather_data.weather[0];

                        let detailed_temp = TemperatureInfo {
                            min: day_weather_data.temp.min as f32,
                            max: day_weather_data.temp.max as f32,

                            morning: day_weather_data.temp.morn as f32,
                            day: day_weather_data.temp.day as f32,
                            evening: day_weather_data.temp.eve as f32,
                            night: day_weather_data.temp.night as f32,
                        };

                        let day_weather_info = WeatherInfo {
                            description: SharedString::from(&weather_details.description),
                            icon_source: SharedString::from(&weather_details.icon),
                            current_temp: day_weather_data.temp.day as f32,
                            detailed_temp: detailed_temp,
                            uv: day_weather_data.uvi as i32,
                            precipitation_prob: day_weather_data.pop as f32,
                            rain: day_weather_data.rain.unwrap_or(0 as f64) as f32,
                            snow: day_weather_data.snow.unwrap_or(0 as f64)  as f32,
                        };

                        // TODO: localization
                        forecast_weather_info.push(WeatherForecastInfo {
                            day_name: SharedString::from(get_day_from_datetime(datetime)),
                            weather_info: day_weather_info
                        });
                    }
                }
            }
        }

        forecast_weather_info
    }

    fn city_weather_info_from_client(city_client: &WeatherClient) -> CityWeatherInfo {
        let current_weather_info = Self::current_weather_info_from_response(&city_client.weather_data);
        let forecast_weather_info = Self::forecast_weather_info_from_response(&city_client.weather_data);

        CityWeatherInfo {
            city_name: SharedString::from(&city_client.city_data.city_name),
            current_weather: current_weather_info,
            forecast_weather: Rc::new(slint::VecModel::from(forecast_weather_info)).into(),
            update_time: SharedString::from(timestamp_to_string(city_client.last_update_timestamp)),
        }
    }
}
