# cargo-sdl-apk
Build Android packages that use SDL.

This cargo tool aims to be for [Rust SDL](https://docs.rs/sdl2/latest/sdl2/) what [cargo-apk](https://crates.io/crates/cargo-apk) is for [Glutin](https://crates.io/crates/glutin), and [cargo-quad-apk](https://crates.io/crates/cargo-quad-apk) is for [Miniquad](https://crates.io/crates/miniquad). That is, a simple command to package up an APK and upload it to your phone and start it. The way it works internally is by automating the steps described [in this article](https://julhe.github.io/posts/building-an-android-app-with-rust-and-sdl2/) by Julian Heinken.

## Basic usage

1. Make sure you have the following:
   * The SDL source, clone it from [here](https://github.com/libsdl-org/SDL). Make sure you have the `release-2.26.x` branch.
   * Java: jdk17
   * Android SDK with command line tools.
   * Android NDK.
2. Set the environment variables:
   * `ANDROID_HOME` pointing to the Android SDK.
   * `ANDROID_NDK_HOME` pointing to the Android NDK.
   * `SDL` pointing to the SDL source dir.
3. Run `cargo run --release -- build --manifest-path /path/to/your/project/Cargo.toml --release`

## Project setup

The entry point for your application must be called `SDL_main` and use the attribute `#[no_mangle]`. Here is an [example project](https://github.com/riseupgroup/cargo-sdl-apk/tree/master/example-project). To build the project, run `cargo run --release -- build --manifest-path ./example-project/Cargo.toml --release`

### Supported `[package.metadata.android]` keys

```toml
title = "Example Project"
package_name = "com.example.example_project"
# You can specify an icon OR an adaptive icon
icon = "icon.png"
adaptive_icon_foreground = "icon_foreground.png"
adaptive_icon_background = "icon_background.png"
adaptive_icon_monochrome = "icon_monochrome.png"
improve_fullscreen = true
```

