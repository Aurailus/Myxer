# Building

Building Myxer is trivial. Download this repository, Cargo, and `libpulse-dev` & `libgtk-3-dev` system libraries, and run `cargo build --release` in the root directory.

## Prebuilt Binaries

Major releases are available on the [Releases](https://github.com/Aurailus/Myxer/releases) page. If you want something more breaking edge, you can download an artifact of the latest commit [here](https://nightly.link/Aurailus/myxer/workflows/release/master/Myxer.zip). These artifacts are untested, YMMV.

## Development 

Call `cargo run` to build and run the application. If you have nodemon installed, you can call it on the root directory to automatically watch the source files for changes and recompile.
