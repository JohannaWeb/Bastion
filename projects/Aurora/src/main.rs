// Module for DOM (Document Object Model) tree representation
// RUST FUNDAMENTAL: Modules organize code into namespaces; 'mod dom;' declares module
// Rust searches for dom in: dom.rs file or dom/mod.rs directory
mod dom;

// Module for CSS stylesheet parsing and cascade rules
// RUST FUNDAMENTAL: Module tree - each mod creates a namespace; accessed via dom::Node, css::Stylesheet, etc.
mod css;

// Module for fetching HTML and other resources from network
// RUST FUNDAMENTAL: Private modules by default; pub mod makes public; pub use re-exports
mod fetch;

// Module for font handling and glyph metrics
// RUST FUNDAMENTAL: Visibility rules: private inside module, pub to expose to parent, pub(crate) crate-wide
mod font;

// Module for texture atlasing and glyph rendering
mod atlas;

// Module for HTML parsing and tokenization
// RUST FUNDAMENTAL: Circular references managed via Rc<RefCell<>> to avoid ownership issues
mod html;

// Module for layout tree generation from styled nodes
// RUST FUNDAMENTAL: Tree structures (DOM, Layout, Style) demonstrate recursive ownership patterns
mod layout;

// Module for painting layout boxes to framebuffer
mod paint;

// Module for styling DOM with CSS rules
// RUST FUNDAMENTAL: StyleTree applies CSS cascade rules to DOM - demonstrates visitor pattern
mod style;

// Module for legacy JavaScript engine (disabled)
// RUST FUNDAMENTAL: #[allow(dead_code)] suppresses warnings for intentionally unused code
mod js;

// Module for Boa JavaScript runtime integration
// RUST FUNDAMENTAL: Boa runtime demonstrates FFI (Foreign Function Interface) pattern in Rust
mod js_boa;

// Module for GPU window rendering and display
// RUST FUNDAMENTAL: Window module uses winit for cross-platform event handling - trait objects pattern
mod window;

// Module for GPU painting capabilities (currently unused)
// RUST FUNDAMENTAL: #[allow(dead_code)] allows keeping prototype code for future use
#[allow(dead_code)]
mod gpu_paint;

// Import CSS stylesheet type for managing style rules
// RUST FUNDAMENTAL: 'use' imports bring items into current namespace; 'crate::' refers to root
use crate::css::Stylesheet;

// Import HTML fetching function for loading remote documents
// RUST FUNDAMENTAL: Functions are items like structs/enums - can be imported with 'use'
use crate::fetch::fetch_html;

// Import HTML parser for converting HTML strings to DOM
// RUST FUNDAMENTAL: Type imports (struct Parser) allow Parser::new() without crate prefix
use crate::html::Parser;

// Import layout tree for spatial box calculations
// RUST FUNDAMENTAL: Each module re-exports public items; use statements resolve them hierarchically
use crate::layout::LayoutTree;

// Import style tree for DOM nodes with applied styles
use crate::style::StyleTree;

// Import PathBuf for file system path manipulation
// RUST FUNDAMENTAL: std:: is Rust's standard library; PathBuf is owned string for paths (like String)
use std::path::PathBuf;

// Import Rc for reference counting shared data
// RUST FUNDAMENTAL: Rc<T> - Reference Counting; enables multiple ownership without garbage collector
// Thread-unsafe but zero-cost; use Arc<T> for thread-safe reference counting
use std::rc::Rc;

// Import capability types and identity from Opus domain model
// RUST FUNDAMENTAL: External crate imports use crate name directly; Cargo.toml controls dependencies
use opus::domain::{Capability, Identity};

// Import environment variable access
// RUST FUNDAMENTAL: std::env provides OS interface; std::env::args() and std::env::var() are common
use std::env;

// Entry point function for Aurora browser
// RUST FUNDAMENTAL: fn main() {} is the program entry point; execution starts here
// Return type: () (unit type, similar to void in C); implicit return of () if no explicit return
fn main() {
    // Print startup message to stdout
    // RUST FUNDAMENTAL: println! is a macro (! suffix); compiles to formatted print code at compile-time
    println!("Aurora: Starting up...");

    // Install rustls crypto provider for TLS operations
    // RUST FUNDAMENTAL: Method chaining - each method returns Self or Option/Result for chaining
    // :: is namespace operator; rustls::crypto::ring is fully qualified path
    rustls::crypto::ring::default_provider()
        // Configure as default crypto backend
        // RUST FUNDAMENTAL: .install_default() consumes self (takes ownership), returns Result<(), Error>
        .install_default()
        // Panic if crypto provider fails to install
        // RUST FUNDAMENTAL: .expect(msg) unwraps Ok(T), or panics with message if Err(E)
        // Used when we know error shouldn't happen, or when failing is appropriate
        .expect("Failed to install rustls crypto provider");
    // Print confirmation that crypto provider loaded
    println!("Aurora: Crypto provider installed.");

    // Create identity tuple with Johanna's capabilities and permissions
    // RUST FUNDAMENTAL: let binding creates variable; type inferred from Identity::new() return
    let identity = Identity::new(
        // Identity URI using decentralized identifier format
        // RUST FUNDAMENTAL: &str literals are string slices (immutable references); &'static str for compile-time strings
        "did:human:johanna",
        // Human-readable name
        "Johanna",
        // Mark this as a human identity type
        // RUST FUNDAMENTAL: :: syntax for enum variants (like Java's Enum.VARIANT)
        opus::domain::IdentityKind::Human,
        // Array of capabilities this identity has: network access and workspace read
        // RUST FUNDAMENTAL: [T; N] is fixed-size array; passed by copy since Copy trait implemented
        // Different from Vec<T> (heap-allocated, growable)
        [Capability::NetworkAccess, Capability::ReadWorkspace],
    );

    // Parse command-line arguments and environment variables into CLI options
    let cli = CliOptions::from_env();

    // Fetch HTML content from URL or use demo HTML if no URL provided
    // RUST FUNDAMENTAL: match expression - exhaustive pattern matching; all arms must return same type
    let html = match cli.input_url.as_deref() {
        // RUST FUNDAMENTAL: Option<T> enum has Some(T) and None variants
        // .as_deref() converts Option<String> to Option<&str> (dereference projection)
        // If URL is provided in arguments
        Some(url) => match fetch_html(url, &identity) {
            // RUST FUNDAMENTAL: Result<T, E> enum has Ok(T) and Err(E) variants
            // fetch_html() returns Result<String, FetchError>
            // If fetch succeeds, use the fetched HTML
            Ok(html) => html,
            // If fetch fails, print error and exit with code 1
            Err(error) => {
                // RUST FUNDAMENTAL: eprintln! prints to stderr (standard error stream)
                // {url} and {error} are interpolated using Display trait formatting
                eprintln!("Failed to fetch {url}: {error}");
                // RUST FUNDAMENTAL: std::process::exit(code) terminates program immediately
                // 0 = success, non-zero = error code
                std::process::exit(1);
            }
        },
        // If no URL provided, use hardcoded demo HTML
        // RUST FUNDAMENTAL: None pattern in match - handled separately from Some
        None => demo_html().to_string(),
    };

    // Clone URL argument for later use in stylesheet parsing
    // RUST FUNDAMENTAL: .clone() creates deep copy of String; needed because html moved into binding above
    // Clone only when necessary - prefer borrowing (&) when possible for performance
    let url_arg = cli.input_url.clone();

    // Set viewport width in pixels (width of rendering canvas)
    // RUST FUNDAMENTAL: 1200.0 is f64 literal (64-bit float); f32 suffix needed for 32-bit float
    // Type inference determines 1200.0_f32 based on function parameter types
    let viewport_width = 1200.0;
    // Parse HTML string into DOM tree structure
    let dom = Parser::new(&html).parse_document();
    // Extract stylesheets from DOM (style tags) and create stylesheet object
    let mut stylesheet = Stylesheet::from_dom(&dom, url_arg.as_deref(), &identity);
    // Merge user agent default styles (browser built-in styles)
    stylesheet.merge(Stylesheet::user_agent_stylesheet());
    // Apply stylesheet rules to DOM tree, creating styled nodes
    let style_tree = StyleTree::from_dom(&dom, &stylesheet);
    // Generate layout boxes from styled tree with calculated positions and sizes
    let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, viewport_width);

    // If DOM debug flag set, print the parsed DOM tree structure
    if cli.debug_dom {
        println!("{}", dom.borrow());
    }
    // If style debug flag set, print the styled tree with applied CSS
    if cli.debug_style {
        println!("{style_tree}");
    }
    // If layout debug flag set, print the layout tree with positions
    if cli.debug_layout {
        println!("{layout}");
    }

    // Extract all script elements (inline and external) from the DOM
    let scripts = extract_scripts(&dom);
    // Only process scripts if any were found
    if !scripts.is_empty() {
        // Print number of scripts being processed
        println!("Boa: Processing {} scripts...", scripts.len());
        // Create new Boa JavaScript runtime, sharing reference to DOM
        let mut runtime = js_boa::BoaRuntime::new(Rc::clone(&dom));
        // Iterate over each script tuple (content/URL, is_external_flag)
        for (source, is_url) in scripts {
            // Determine script content by loading from URL or using inline
            let script_content = if is_url {
                // If script has a src attribute, treat as external URL
                if let Some(base) = url_arg.as_deref() {
                    // Resolve relative URLs against document base URL
                    match crate::fetch::resolve_relative_url(base, &source) {
                        // If resolution succeeds, fetch the script content
                        Ok(full_url) => {
                            // Print the URL being fetched
                            println!("Boa: Fetching external script: {}", full_url);
                            // Fetch the script file over network
                            match crate::fetch::fetch_string(&full_url, &identity) {
                                // Use fetched content if successful
                                Ok(content) => content,
                                // Skip script if fetch fails
                                Err(e) => {
                                    eprintln!("Failed to fetch script {}: {}", full_url, e);
                                    continue;
                                }
                            }
                        }
                        // Skip script if URL resolution fails
                        Err(e) => {
                            eprintln!("Failed to resolve script URL {}: {}", source, e);
                            continue;
                        }
                    }
                } else {
                    // Skip external script if no base URL available
                    continue;
                }
            } else {
                // Use inline script content directly
                source
            };

            // Execute script in Boa runtime with error handling
            if let Err(e) = runtime.execute(&script_content) {
                // Print JavaScript execution errors to stderr
                eprintln!("JS Error: {}", e);
            }
        }
    }

    // Initialize font atlas before rendering (forces glyph load)
    // Get metrics for sample character to populate atlas
    let _ = crate::font::get_glyph_metrics('A');

    // Check environment variables to determine rendering mode
    // Check if screenshot output path is specified in environment
    let has_screenshot = env::var("AURORA_SCREENSHOT").is_ok();
    // Check if headless mode is explicitly requested
    let is_headless = env::var("AURORA_HEADLESS").is_ok();
    // Check if X11 or Wayland display server is available
    let has_display = env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok();

    // Decide whether to attempt window rendering
    if has_screenshot || (!is_headless && has_display) {
        // Attempt to open interactive GPU window for rendering
        if let Err(error) = window::open(&layout) {
            // Print error if window creation fails
            eprintln!("Window disabled: {error}");
            // Suggest alternative output methods to user
            eprintln!("Set AURORA_SCREENSHOT=/path/output.png for file output or AURORA_HEADLESS=1 to skip window creation.");
        }
    } else if !is_headless && !has_display {
        // Print message if no display server detected (and not explicitly headless)
        eprintln!("No display server detected; skipping window creation.");
        // Suggest screenshot mode as alternative
        eprintln!("Set AURORA_SCREENSHOT=/path/output.png for file output.");
    } else {
        // Print message confirming headless mode is active
        eprintln!("Headless mode: skipping window");
    }
}

// Structure holding parsed command-line and environment options
// RUST FUNDAMENTAL: #[derive(Debug, Clone)] - derives generate implementations for us
// Debug - enables {:?} formatting; Clone - auto-generates clone() method
// Avoid #[derive(Copy)] for types containing String (Copy requires no heap allocation)
#[derive(Debug, Clone)]
struct CliOptions {
    // Optional URL to fetch and render as input
    // RUST FUNDAMENTAL: Option<T> = Some(T) | None; safe null handling avoiding null pointer errors
    // Encourages handling both cases explicitly via match or .map()
    input_url: Option<String>,

    // Flag to print DOM tree after parsing
    // RUST FUNDAMENTAL: bool is primitive (Copy); immutable by default; use mut for mutability
    debug_dom: bool,

    // Flag to print styled tree after CSS application
    debug_style: bool,

    // Flag to print layout tree after position calculations
    debug_layout: bool,
}

// Implementation of CliOptions parsing from command-line args
// RUST FUNDAMENTAL: impl allows adding methods to types; separate from struct definition
// Associated functions (no self) called with CliOptions::from_env(); methods need &self
impl CliOptions {
    // Parse environment variables and command-line arguments
    // RUST FUNDAMENTAL: fn from_env() -> Self is constructor pattern; returns instance of Self (CliOptions)
    fn from_env() -> Self {
        // Skip program name, iterate over remaining arguments
        // RUST FUNDAMENTAL: env::args() returns iterator over String; .skip(1) skips argv[0]
        // Iterators are lazy - chain operations without allocating intermediate collections
        let mut args = env::args().skip(1);

        // Start with no input URL
        // RUST FUNDAMENTAL: let mut makes variable mutable; necessary for args.next() and reassignments
        let mut input_url = None;

        // Check for debug flags in environment variables
        // RUST FUNDAMENTAL: Function calls initialize with function return value
        let mut debug_dom = env_flag("AURORA_DEBUG_DOM");

        // Check environment for debug style flag
        let mut debug_style = env_flag("AURORA_DEBUG_STYLE");

        // Check environment for debug layout flag
        let mut debug_layout = env_flag("AURORA_DEBUG_LAYOUT");

        // Process each command-line argument
        // RUST FUNDAMENTAL: while let Some(arg) = iterator.next() unwraps Option in loop condition
        // Cleaner than traditional while loops; stops when iterator returns None
        while let Some(arg) = args.next() {
            // Match argument against known flags and values
            // RUST FUNDAMENTAL: match arg.as_str() pattern match on enum/string; all cases must be handled
            // Compiler ensures exhaustiveness - misses = compile error (safe by default)
            match arg.as_str() {
                // Fixture mode: load HTML from fixtures directory
                "--fixture" => {
                    // Get the next argument as fixture name
                    // RUST FUNDAMENTAL: let Some(name) = args.next() else { error } is let-else pattern
                    // Introduced in Rust 1.65; destructures Option with fallback for None case
                    let Some(name) = args.next() else {
                        // Error if no name provided after flag
                        eprintln!("Missing fixture name after --fixture");
                        // Exit with error code
                        std::process::exit(2);
                    };
                    // Construct file URL to fixture HTML
                    input_url = Some(fixture_url(&name));
                }

                // Override debug_dom with command-line flag
                "--debug-dom" => debug_dom = true,

                // Override debug_style with command-line flag
                "--debug-style" => debug_style = true,

                // Override debug_layout with command-line flag
                "--debug-layout" => debug_layout = true,

                // Reject unknown flags starting with --
                // RUST FUNDAMENTAL: if guard (if other.starts_with("--")) refines pattern matching
                // Matches when pattern and guard both true
                other if other.starts_with("--") => {
                    eprintln!("Unknown flag: {other}");
                    std::process::exit(2);
                }

                // Treat other arguments as input URL
                // RUST FUNDAMENTAL: _ is catch-all pattern; other captures the value for use
                // Pattern is evaluated in order - must come after specific cases
                other => {
                    input_url = Some(other.to_string());
                }
            }
        }

        // Return populated CliOptions struct
        // RUST FUNDAMENTAL: Self refers to the type being implemented on (CliOptions here)
        // Uses shorthand struct construction: Self { field1, field2 } instead of CliOptions { ... }
        // All fields must be initialized; compiler checks for missing fields
        Self {
            input_url,
            debug_dom,
            debug_style,
            debug_layout,
        }
    }
}

// Check if environment variable is set to a truthy value
// RUST FUNDAMENTAL: Functions decompose logic; pure functions (deterministic, side-effect-free) preferred
fn env_flag(name: &str) -> bool {
    // Get environment variable and check if it matches any true values
    // RUST FUNDAMENTAL: matches! macro - shorthand for pattern matching boolean result
    // Equivalent to: match env::var(name).as_deref() { Ok("1") | ... => true, _ => false }
    matches!(
        // Read environment variable and dereference for comparison
        // RUST FUNDAMENTAL: env::var() returns Result<String, VarError>
        // .as_deref() converts Result<String, E> to Result<&str, E> (borrows instead of moving)
        // Allows pattern matching on &str without consuming the String
        env::var(name).as_deref(),

        // Match "1", "true" (any case), or "yes" (any case)
        // RUST FUNDAMENTAL: | is OR pattern - matches if any arm matches
        // Ok(...) unwraps Ok variant, literal string patterns match exactly
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

// Construct file:// URL to a test fixture HTML file
fn fixture_url(name: &str) -> String {
    // Get the Cargo manifest directory (project root) at compile time
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Add fixtures subdirectory
    path.push("fixtures");
    // Add fixture name directory
    path.push(name);
    // Add index.html as the document to load
    path.push("index.html");
    // Format as file:// URL with absolute path
    format!("file://{}", path.display())
}

// Return hardcoded HTML demo content for default rendering
fn demo_html() -> &'static str {
    // Return static string containing complete HTML document
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

// Extract all script tags from DOM and return (content/src, is_external) tuples
// RUST FUNDAMENTAL: Function returns Vec<(String, bool)> - vector of tuples
// Tuple (String, bool) represents (script_content_or_url, is_external_flag)
fn extract_scripts(node: &crate::dom::NodePtr) -> Vec<(String, bool)> {
    // Initialize result vector to collect script tuples
    // RUST FUNDAMENTAL: Vec::new() creates empty heap-allocated vector; grows as needed
    let mut scripts = Vec::new();

    // Define nested walk function to traverse DOM tree recursively
    // RUST FUNDAMENTAL: Inner function (nested function) captures scripts via mutable parameter
    // &mut Vec<...> allows walk() to modify scripts; by-reference avoids copying Vector
    fn walk(node: &crate::dom::NodePtr, scripts: &mut Vec<(String, bool)>) {
        // Borrow the node to access its data
        // RUST FUNDAMENTAL: .borrow() returns Ref<T> (shared borrow); single reader allowed
        // Opposite of .borrow_mut() which panics if anyone else holds a borrow
        let node_borrow = node.borrow();

        // Match on node type
        // RUST FUNDAMENTAL: &*node_borrow dereferences Ref<Node> to &Node for pattern matching
        // Pattern matching exhaustiveness checked at compile-time; all variants must be handled
        match &*node_borrow {
            // If node is a script element
            // RUST FUNDAMENTAL: if guard (if el.tag_name == "script") refines pattern
            // Match only if both pattern matches AND condition is true
            crate::dom::Node::Element(el) if el.tag_name == "script" => {
                // Check if script has src attribute (external script)
                // RUST FUNDAMENTAL: .get(key) returns Option<&V>; if let unwraps Some
                if let Some(src) = el.attributes.get("src") {
                    // Add src URL as external script (true flag)
                    // RUST FUNDAMENTAL: .push((src.clone(), true)) adds tuple to vector
                    // .clone() needed because src is &String (borrowed), tuple needs owned String
                    scripts.push((src.clone(), true));
                } else {
                    // No src attribute means inline script
                    // Collect text content from script children
                    // RUST FUNDAMENTAL: String::new() creates empty owned String
                    let mut content = String::new();

                    // Iterate through script tag's children
                    // RUST FUNDAMENTAL: for loop borrows el.children; iteration consumes no ownership
                    for child in &el.children {
                        // Borrow child to check its type
                        let child_borrow = child.borrow();

                        // If child is text node, append to content
                        // RUST FUNDAMENTAL: if let pattern matches Text variant, destructures String
                        if let crate::dom::Node::Text(t) = &*child_borrow {
                            // RUST FUNDAMENTAL: .push_str() appends &str to String (doesn't take ownership)
                            content.push_str(t);
                        }
                    }

                    // Only add script if it has content
                    if !content.is_empty() {
                        // Add script content as inline script (false flag)
                        // RUST FUNDAMENTAL: .push() moves content into Vec (ownership transfer)
                        scripts.push((content, false));
                    }
                }
            }

            // If node is any other element, recurse into children
            // RUST FUNDAMENTAL: Catch-all pattern for non-script elements
            crate::dom::Node::Element(el) => {
                // Walk each child of this element
                // RUST FUNDAMENTAL: for loop over mutable collection doesn't require &mut
                // Iteration by reference (&el.children) is default
                for child in &el.children {
                    // Recursively process child nodes
                    // RUST FUNDAMENTAL: Recursive call with same signature
                    // Stack depth = tree depth; deep trees might overflow stack (could use iterative approach)
                    walk(child, scripts);
                }
            }

            // If node is document root, recurse into its children
            // RUST FUNDAMENTAL: Struct variant with named field; destructure with { children }
            crate::dom::Node::Document { children } => {
                // Walk each top-level child
                // RUST FUNDAMENTAL: Iteration over &[NodePtr] (slice reference)
                for child in children {
                    // Recursively process child nodes
                    walk(child, scripts);
                }
            }

            // Ignore text nodes (no scripts there)
            // RUST FUNDAMENTAL: _ is catch-all pattern; ignores the value
            _ => {}
        }
    }

    // Start recursive walk from the given node
    // RUST FUNDAMENTAL: Call inner walk() function; passes &mut scripts
    walk(node, &mut scripts);

    // Return collected scripts vector
    // RUST FUNDAMENTAL: Function returns scripts; moves ownership to caller
    scripts
}
