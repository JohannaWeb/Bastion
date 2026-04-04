# Gisberta

`Gisberta` is a custom browser build based on Servo's `servoshell`.

This directory currently contains only the top-level README. The actual Servo source tree referenced below is expected to live elsewhere in the developer's workspace.

## Build

From the Servo checkout used for this project:

```bash
cargo run -p servoshell --bin gisberta
```

## What Changed

- The browser chrome in `servoshell` now uses a pastel pink theme.
- The window title, app metadata, and binary name are branded as `Gisberta`.
- The built-in `servo:newtab` page was rewritten with custom search and quick links.

## Notes

- This project uses Servo itself rather than a wrapper around another browser engine.
- Building Servo can require system libraries depending on platform, especially for desktop graphics and media.
