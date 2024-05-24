# Rusty Weather

Rusty Weather is a simple weather application. It's a demo project made by [Felgo](https://felgo.com/) at the start of our journey with Rust and Slint.

The app was visually based on an existing Felgo project ([Weather Pro](https://felgo.com/weather-app)), but it's not exactly the same, so we could have played with the language features we were interested in.

The project is targeted at both desktops and Android devices.

## Usage
Set the `RUSTY_WEATHER_API_KEY` environment variable to your [OpenWeatherMap](https://openweathermap.org/) API key. The OneCall API subscription is required. 

### Compiling on desktop
1. Build with cargo:
    ```
    cargo build
    ```
   
3. Run the application binary:
     ```
     cargo run
     ```

### Compiling on Android
To be able to compile the application for Android, the [cargo apk](https://crates.io/crates/cargo-apk) crate is required.

1. Provide rust for the android target:
    ```
    rustup target add aarch64-linux-android
    ```
   
2. Install cargo-apk:
    ```
    cargo install cargo-apk
    ```
   
3. Build with cargo:
    ```
    cargo apk build --package rusty-weather --lib
    ```
   
   * To build open-ssl for Android, the proper development libraries need to be available on the system,
     and paths to the NDK binaries available in the file system.


4. Run the application:
     ```
     cargo apk run --package rusty-weather --lib
     ```
