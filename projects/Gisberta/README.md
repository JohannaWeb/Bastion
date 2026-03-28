# Gisberta

`Gisberta` is a pastel pink browser build based on Servo's official `servoshell`, written in Rust.

The project name honors Gisberta Salce Junior, a trans woman murdered in 2006. This build keeps that tribute visible in the new-tab page and browser branding.

## Build

From [`vendor/servo`](/home/johanna/projects/Gisberta/vendor/servo):

```bash
cargo run -p servoshell --bin gisberta
```

## What Changed

- The browser chrome in `servoshell` now uses a pastel pink theme.
- The window title, app metadata, and binary name are branded as `Gisberta`.
- The built-in `servo:newtab` page was rewritten as a tribute landing page with search and quick links.

## Notes

- This repo vendors Servo directly so the browser is using the actual Servo engine, not a placeholder wrapper.
- Building Servo can require system libraries depending on platform, especially for desktop graphics and media.
