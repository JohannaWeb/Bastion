mod dom;
mod css;
mod fetch;
mod font;
mod atlas;
mod html;
mod layout;
mod paint;
mod style;
mod js;
mod js_boa;
mod window;
#[allow(dead_code)]
mod gpu_paint;

use crate::css::Stylesheet;
use crate::fetch::fetch_html;
use crate::html::Parser;
use crate::layout::LayoutTree;
use crate::paint::Painter;
use crate::style::StyleTree;
use std::rc::Rc;
use opus::domain::{Capability, Identity};
use std::env;

fn main() {
    println!("Aurora: Starting up...");
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    println!("Aurora: Crypto provider installed.");

    let identity = Identity::new(
        "did:human:johanna",
        "Johanna",
        opus::domain::IdentityKind::Human,
        [Capability::NetworkAccess, Capability::ReadWorkspace],
    );

    let html = match env::args().nth(1) {
        Some(url) => match fetch_html(&url, &identity) {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Failed to fetch {url}: {error}");
                std::process::exit(1);
            }
        },
        None => demo_html().to_string(),
    };

    let url_arg = env::args().nth(1);
    let viewport_width = 1200.0;
    let dom = Parser::new(&html).parse_document();
    let mut stylesheet = Stylesheet::from_dom(&dom, url_arg.as_deref(), &identity);
    stylesheet.merge(Stylesheet::user_agent_stylesheet());
    let style_tree = StyleTree::from_dom(&dom, &stylesheet);
    let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, viewport_width);

    // Boa JS Integration
    let scripts = extract_scripts(&dom);
    if !scripts.is_empty() {
        println!("Boa: Processing {} scripts...", scripts.len());
        let mut runtime = js_boa::BoaRuntime::new(Rc::clone(&dom));
        for (source, is_url) in scripts {
            let script_content = if is_url {
                if let Some(base) = url_arg.as_deref() {
                    match crate::fetch::resolve_relative_url(base, &source) {
                        Ok(full_url) => {
                            println!("Boa: Fetching external script: {}", full_url);
                            match crate::fetch::fetch_string(&full_url, &identity) {
                                Ok(content) => content,
                                Err(e) => {
                                    eprintln!("Failed to fetch script {}: {}", full_url, e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to resolve script URL {}: {}", source, e);
                            continue;
                        }
                    }
                } else {
                    continue;
                }
            } else {
                source
            };

            if let Err(e) = runtime.execute(&script_content) {
                eprintln!("JS Error: {}", e);
            }
        }
    }

    // Open GPU window for rendering
    println!("{layout}");

    // Initialize font atlas early
    let _ = crate::font::get_glyph_metrics('A');

    // Check if we need to render output (screenshot or interactive window)
    let has_screenshot = env::var("AURORA_SCREENSHOT").is_ok();
    let is_headless = env::var("AURORA_HEADLESS").is_ok();

    if has_screenshot || !is_headless {
        window::open(&layout);
    } else {
        eprintln!("Headless mode: skipping window");
    }
}

fn demo_html() -> &'static str {
    r#"
        <html>
            <head>
                <style>
                    h1 { color: #d26428; font-weight: bold; font-size: 48px; }
                    h2 { color: #2E3440; font-size: 32px; margin-top: 20px; }
                    p { font-size: 20px; }
                    code { color: #BF616A; font-size: 20px; }
                </style>
            </head>
            <body>
                <h1>Aurora Browser - Unicode & Symbol Test</h1>

                <h2>Basic Typography</h2>
                <p>This paragraph has multiple words that wrap across lines and includes <strong>bold text</strong> and <em>italic text</em> interspersed throughout to test proper spacing preservation.</p>

                <h2>Unicode Symbols</h2>
                <p>Weather: ☀ sun ☁ cloud ☂ umbrella ☃ snowman</p>
                <p>Stars: ★ filled ☆ empty ☇ comet</p>
                <p>Arrows: ← → ↑ ↓ ↔ ↕</p>
                <p>Math: ± × ÷ ≈ ≠ ≡ ∞</p>

                <h2>Box Drawing</h2>
                <p>─ horizontal bar │ vertical bar</p>
                <p>┌─┐ ├─┤ └─┘ box corners and tees</p>
                <p>┼ cross symbol</p>

                <h2>Special Characters</h2>
                <p>Symbols: © ® ° · – —</p>
                <p>Bullets: • ◦ ‣</p>
                <p>Degrees: 32° F = 0° C</p>

                <h2>Mixed Content</h2>
                <p>Temperature: 72° Status: ☀ Clear skies with ← wind from west.</p>
                <p>Box: ┌─────┐ filled │ with │ ├─────┤ lines └─────┘</p>
            </body>
        </html>
    "#
}

fn extract_scripts(node: &crate::dom::NodePtr) -> Vec<(String, bool)> {
    let mut scripts = Vec::new();
    fn walk(node: &crate::dom::NodePtr, scripts: &mut Vec<(String, bool)>) {
        let node_borrow = node.borrow();
        match &*node_borrow {
            crate::dom::Node::Element(el) if el.tag_name == "script" => {
                if let Some(src) = el.attributes.get("src") {
                    scripts.push((src.clone(), true));
                } else {
                    let mut content = String::new();
                    for child in &el.children {
                        let child_borrow = child.borrow();
                        if let crate::dom::Node::Text(t) = &*child_borrow {
                            content.push_str(t);
                        }
                    }
                    if !content.is_empty() {
                        scripts.push((content, false));
                    }
                }
            }
            crate::dom::Node::Element(el) => {
                for child in &el.children {
                    walk(child, scripts);
                }
            }
            crate::dom::Node::Document { children } => {
                for child in children {
                    walk(child, scripts);
                }
            }
            _ => {}
        }
    }
    walk(node, &mut scripts);
    scripts
}
