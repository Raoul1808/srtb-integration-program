# SRTB Integration Program

A tool used to integrate modchart data into SRTB files.

- [Download link](https://github.com/Raoul1808/srtb-integration-program/releases)
- [WASM Version](https://raoul1808.github.io/srtb-integration-program) (doesn't work on Chrome and Chromium-based browsers at the moment)

This project was initially hosted on the [Dynamic Track Speed repository](https://github.com/Raoul1808/DynamicTrackSpeed), but I decided to move the project to a different repository to avoid overwhelming the history with the integration tool.

## Building the project

Pre-requisites:
- An up-to-date Rust toolchain (preferably installed with [rustup](https://rustup.rs/))
- If you run Linux, you may need to install a gtk-dev package (see workflows)

Steps:
1. Clone this repo
2. Run `cargo build` or `cargo run --package <srtb-integration-cli|srtb-integration-gui>` in a terminal
3. Profit

## Building the WebAssembly gui app

Pre-requisites:
- Same as building the project
- The `wasm32-unknown-unknown` Rust toolchain installed
- [Trunk](https://trunkrs.dev)

Steps:
1. Clone this repo
2. cd into the `srtb-integration-gui` directory
3. Run `trunk serve`
4. Connect to `http://localhost:8080`
5. Profit

## License

This project is licensed under the [MIT License](LICENSE)
