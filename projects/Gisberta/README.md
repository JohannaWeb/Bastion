# Gisberta

`Gisberta` is a custom browser build based on Servo's official `servoshell`, written in Rust.

## Build

From [`vendor/servo`](/home/johanna/projects/Gisberta/vendor/servo):

```bash
cargo run -p servoshell --bin gisberta
```

## What Changed

- The browser chrome in `servoshell` now uses a pastel pink theme.
- The window title, app metadata, and binary name are branded as `Gisberta`.
- The built-in `servo:newtab` page was rewritten with custom search and quick links.

## Notes

- This repo vendors Servo directly so the browser is using the actual Servo engine, not a placeholder wrapper.
- Building Servo can require system libraries depending on platform, especially for desktop graphics and media.
