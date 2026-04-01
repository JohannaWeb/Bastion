# Aurora — Sovereign Browser Engine

Aurora is the flagship Rust browser engine for the **Bastion sovereign developer stack**. It is a from-scratch implementation designed for an owned client surface, free from Chromium or traditional WebView dependencies.

## The Thesis

In a world where agents act on behalf of users, the browser must be more than a renderer; it must be a trusted termination point for protocols. Aurora is built to eventually integrate:

- **DID-Native Identity**: Identity resolution built into the browser's core.
- **AT Protocol Integration**: Native support for decentralized coordination.
- **Sovereign Render Path**: A GPU-accelerated rendering pipeline owned by the user.

Current experimental slice:

- tokenize a narrow HTML subset
- build a simple DOM tree
- extract and parse a tiny CSS subset from `<style>` tags
- match tag, `.class`, `#id`, and descendant rules into computed styles
- build a style tree with basic color inheritance
- derive a block-oriented layout tree
- paint the result into a tiny text framebuffer
- print both structures from a CLI binary

## Run

```bash
cargo run
```

To fetch a page over the network:

```bash
cargo run -- http://example.com/
```

Current fetch support is intentionally small:

- `http://` and `https://`
- basic redirects
- remote images render as placeholders using `<img>` layout and alt text

## Test

```bash
cargo test
```

## Next steps

1. Add a real tokenizer state machine.
2. Add more inherited properties and better CSS value handling.
3. Improve layout with inline flow, wrapping, and margins.
4. Replace the text framebuffer with a real raster or window backend.
