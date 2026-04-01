mod dom;
mod css;
mod fetch;
mod font;
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
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

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
        println!("Boa: Executing {} scripts...", scripts.len());
        let mut runtime = js_boa::BoaRuntime::new(Rc::clone(&dom));
        for script_source in scripts {
            if let Err(e) = runtime.execute(&script_source) {
                eprintln!("JS Error: {}", e);
            }
        }

        // Re-layout after potential DOM mutations
        // (Note: For this session we focus on initial render and one-shot execution)
    }

    // Render using text framebuffer (works great!)
    let framebuffer = Painter::paint(&layout);
    println!("{}", framebuffer);
}

fn demo_html() -> &'static str {
    r#"
        <html>
            <head>
                <style>
                    h1 { color: #d26428; font-weight: bold; }
                </style>
            </head>
            <body>
                <h1>Text Wrapping with Inline Elements</h1>
                <p>This paragraph has multiple words that wrap across lines and includes <strong>bold text</strong> and <em>italic text</em> interspersed throughout to test proper spacing preservation.</p>
                <p>Another test: <strong>Start</strong> with bold, <em>then</em> italic, and finally <strong>more bold</strong> at the end.</p>
            </body>
        </html>
    "#
}

fn extract_scripts(node: &crate::dom::NodePtr) -> Vec<String> {
    let mut scripts = Vec::new();
    fn walk(node: &crate::dom::NodePtr, scripts: &mut Vec<String>) {
        let node_borrow = node.borrow();
        match &*node_borrow {
            crate::dom::Node::Element(el) if el.tag_name == "script" => {
                let mut content = String::new();
                for child in &el.children {
                    let child_borrow = child.borrow();
                    if let crate::dom::Node::Text(t) = &*child_borrow {
                        content.push_str(t);
                    }
                }
                if !content.is_empty() {
                    scripts.push(content);
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
