// Import DOM node types
// RUST FUNDAMENTAL: `NodePtr` is the DOM's shared pointer type alias.
// It hides the `Rc<RefCell<Node>>` machinery so this module can talk about nodes at a higher level.
use crate::dom::{Node, NodePtr};

// Import Boa JavaScript engine
// RUST FUNDAMENTAL: Boa is an embeddable JavaScript engine exposed through Rust APIs.
// The broader idea here is "host one runtime inside another system", where Rust code provides values and functions to the JS world.
// `Context` is Boa's main execution state object, holding globals, heap-managed values, and runtime configuration.
use boa_engine::{Context, JsResult, JsValue, NativeFunction, Source, JsString};

// Import Boa object builder
// RUST FUNDAMENTAL: The builder pattern is common when constructing objects that have many optional pieces.
// Instead of one giant constructor, you chain configuration calls and finish with `build()`.
use boa_engine::object::ObjectInitializer;

// Import property attributes (used below)
// RUST FUNDAMENTAL: Property attributes are metadata attached to object properties.
// They control whether a property can be changed, iterated over, deleted, and so on.
use boa_engine::property::Attribute;

// Import Boa garbage collection traits
// RUST FUNDAMENTAL: Boa uses garbage collection for JS-managed values, so native Rust types participating in that world
// may need to implement GC-related traits. `Trace` tells the collector what references need to be followed.
use boa_gc::{Trace, Finalize, empty_trace};

// Import collections for storing nodes
use std::collections::BTreeMap;

// Import Rc for shared references
// RUST FUNDAMENTAL: `Rc<T>` is ideal when several Rust values need to point at the same data in single-threaded code.
// Cloning an `Rc` is cheap because it copies the pointer and bumps the reference count instead of cloning the underlying value.
use std::rc::Rc;

// Import RefCell for interior mutability
// RUST FUNDAMENTAL: `RefCell<T>` lets code borrow mutably even when the `RefCell` itself is behind shared ownership.
// The price is that borrow-rule violations become runtime panics instead of compile-time errors.
use std::cell::RefCell;

// Registry mapping JavaScript object IDs to DOM nodes
#[derive(Clone)]
struct NodeRegistry {
    // Map of object ID to DOM node pointers
    nodes: Rc<RefCell<BTreeMap<u32, NodePtr>>>,
    // Counter for assigning unique IDs
    next_id: Rc<RefCell<u32>>,
}

// Implement garbage collection traits for registry
unsafe impl Trace for NodeRegistry {
    // Empty trace: Rc/RefCell are not traced in Boa
    empty_trace!();
}
// Implement finalizer (no cleanup needed)
impl Finalize for NodeRegistry {}

// Captured reference to a DOM node in JavaScript context
#[derive(Clone)]
struct NodeCapture {
    // Reference to DOM node
    node: NodePtr,
    // Reference to node registry
    registry: NodeRegistry,
}

// Implement garbage collection traits for node capture
unsafe impl Trace for NodeCapture {
    // Empty trace for this native type
    empty_trace!();
}
// Implement finalizer
impl Finalize for NodeCapture {}

// Captured reference to document node in JavaScript
#[derive(Clone)]
struct DocCapture {
    // Reference to document root node
    document: NodePtr,
    // Reference to node registry
    registry: NodeRegistry,
}

// Implement garbage collection traits for document capture
unsafe impl Trace for DocCapture {
    // Empty trace for this native type
    empty_trace!();
}
// Implement finalizer
impl Finalize for DocCapture {}

// Boa JavaScript runtime wrapping context and DOM
pub struct BoaRuntime {
    // JavaScript execution context
    context: Context,
    // Document node (kept but not directly used)
    #[allow(dead_code)]
    document: NodePtr,
    // Registry mapping JS objects to DOM nodes
    registry: NodeRegistry,
}

impl BoaRuntime {
    pub fn new(document: NodePtr) -> Self {
        let mut context = Context::default();
        let registry = NodeRegistry {
            nodes: Rc::new(RefCell::new(BTreeMap::new())),
            next_id: Rc::new(RefCell::new(1)),
        };
        
        // ... (XHR polyfill same as before) ...
        let xhr_polyfill = r#"
            globalThis.XMLHttpRequest = function() {
                this.readyState = 0;
                this.status = 0;
                this.responseText = "";
                this.onreadystatechange = null;
                this.onload = null;
            };
            globalThis.XMLHttpRequest.prototype.open = function(method, url) {
                this._method = method;
                this._url = url;
                this.readyState = 1;
            };
            globalThis.XMLHttpRequest.prototype.send = function() {
                this.readyState = 4;
                this.status = 200;
                this.responseText = "{}";
                if (typeof this.onreadystatechange === 'function') {
                    this.onreadystatechange();
                }
                if (typeof this.onload === 'function') {
                    this.onload();
                }
            };
            globalThis.XMLHttpRequest.prototype.setRequestHeader = function() {};
        "#;
        let _ = context.eval(Source::from_bytes(xhr_polyfill.as_bytes()));
        
        // Console.log
        let console = ObjectInitializer::new(&mut context)
            .function(NativeFunction::from_fn_ptr(|_this, args, _context| {
                let msg = args.get(0).map(|v| v.display().to_string()).unwrap_or_default();
                println!("JS Console: {}", msg);
                Ok(JsValue::undefined())
            }), JsString::from("log"), 1)
            .build();
        let _ = context.register_global_property(JsString::from("console"), console, Attribute::all());

        // Event listener no-ops for window/document
        let add_event_listener = NativeFunction::from_fn_ptr(|_this, _args, _context| Ok(JsValue::undefined()));
        let dummy = ObjectInitializer::new(&mut context)
            .function(add_event_listener.clone(), JsString::from("f"), 2)
            .build();
        let add_event_listener_js = dummy.get(JsString::from("f"), &mut context).unwrap();

        // Document
        let doc_capture = DocCapture { document: document.clone(), registry: registry.clone() };
        let doc_obj = ObjectInitializer::new(&mut context)
            .function(NativeFunction::from_copy_closure_with_captures(
                |_this, args, captures, context| {
                    let id = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                    if let Some(node) = find_by_id(&captures.document, &id) {
                         Ok(create_js_node(node, &captures.registry, context))
                    } else {
                        Ok(JsValue::null())
                    }
                },
                doc_capture.clone()
            ), JsString::from("getElementById"), 1)
            .function(NativeFunction::from_copy_closure_with_captures(
                |_this, args, captures, context| {
                    let tag = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                    let node = Node::element(tag, vec![]);
                    Ok(create_js_node(node, &captures.registry, context))
                },
                doc_capture.clone()
            ), JsString::from("createElement"), 1)
            .function(NativeFunction::from_copy_closure_with_captures(
                |_this, _args, captures, context| {
                    Ok(create_js_node(captures.document.clone(), &captures.registry, context))
                },
                doc_capture
            ), JsString::from("documentElement"), 0)
            .function(NativeFunction::from_fn_ptr(|_this, args, _context| {
                let text = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                Ok(JsValue::from(JsString::from(text)))
            }), JsString::from("createTextNode"), 1)
            .function(add_event_listener.clone(), JsString::from("addEventListener"), 2)
            .function(add_event_listener.clone(), JsString::from("removeEventListener"), 2)
            .build();
        let _ = context.register_global_property(JsString::from("document"), doc_obj, Attribute::all());

        // Window
        let global_obj = context.global_object().clone();
        let _ = context.register_global_property(JsString::from("window"), global_obj.clone(), Attribute::all());
        let _ = context.register_global_property(JsString::from("global"), global_obj.clone(), Attribute::all());
        
        let _ = global_obj.set(JsString::from("addEventListener"), add_event_listener_js.clone(), false, &mut context);
        let _ = global_obj.set(JsString::from("removeEventListener"), add_event_listener_js, false, &mut context);

        let location = ObjectInitializer::new(&mut context)
            .property(JsString::from("href"), JsValue::from(JsString::from("http://localhost/")), Attribute::all())
            .property(JsString::from("pathname"), JsValue::from(JsString::from("/")), Attribute::all())
            .build();
        let _ = context.register_global_property(JsString::from("location"), location, Attribute::all());

        let navigator = ObjectInitializer::new(&mut context)
            .property(JsString::from("userAgent"), JsValue::from(JsString::from("Aurora/0.1")), Attribute::all())
            .build();
        let _ = context.register_global_property(JsString::from("navigator"), navigator, Attribute::all());

        Self { context, document, registry }
    }

    pub fn execute(&mut self, script: &str) -> JsResult<JsValue> {
        self.context.eval(Source::from_bytes(script))
    }
}

fn create_js_node(node: NodePtr, registry: &NodeRegistry, context: &mut Context) -> JsValue {
    let id = {
        let mut next_id = registry.next_id.borrow_mut();
        let id = *next_id;
        *next_id += 1;
        registry.nodes.borrow_mut().insert(id, node.clone());
        id
    };

    let capture = NodeCapture { node: node.clone(), registry: registry.clone() };

    ObjectInitializer::new(context)
        .property(JsString::from("__node_id"), id, Attribute::READONLY | Attribute::NON_ENUMERABLE)
        .function(NativeFunction::from_copy_closure_with_captures(
            |_this, args, captures, _context| {
                let parent_ptr = &captures.node;
                let registry = &captures.registry;
                let child_js = args.get(0).cloned().unwrap_or(JsValue::null());
                
                if let Some(child_id_val) = child_js.as_object().and_then(|o| o.get(JsString::from("__node_id"), _context).ok()) {
                    if let Some(child_id) = child_id_val.as_number() {
                        if let Some(child_ptr) = registry.nodes.borrow().get(&(child_id as u32)).cloned() {
                            let mut parent = parent_ptr.borrow_mut();
                            if let Node::Element(el) = &mut *parent {
                                el.children.push(child_ptr);
                            }
                        }
                    }
                }
                Ok(child_js)
            },
            capture.clone()
        ), JsString::from("appendChild"), 1)
        .function(NativeFunction::from_copy_closure_with_captures(
            |_this, args, captures, _context| {
                let name = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                let value = args.get(1).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                let mut n = captures.node.borrow_mut();
                if let Node::Element(el) = &mut *n {
                    el.attributes.insert(name, value);
                }
                Ok(JsValue::undefined())
            },
            capture
        ), JsString::from("setAttribute"), 2)
        .build()
        .into()
}

fn find_by_id(node: &NodePtr, id: &str) -> Option<NodePtr> {
   let borrow = node.borrow();
   match &*borrow {
       Node::Element(el) => {
           if el.attributes.get("id").map(|s| s.as_str()) == Some(id) {
               drop(borrow);
               return Some(node.clone());
           }
           for child in &el.children {
               if let Some(found) = find_by_id(child, id) {
                   return Some(found);
               }
           }
           None
       }
       Node::Document { children } => {
           for child in children {
               if let Some(found) = find_by_id(child, id) {
                   return Some(found);
               }
           }
           None
       }
       _ => None
   }
}
