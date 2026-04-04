// Import DOM node types
// RUST FUNDAMENTAL: NodePtr = Rc<RefCell<Node>>; smart pointer for cyclic graph (DOM tree)
use crate::dom::{Node, NodePtr};

// Import Boa JavaScript engine
// RUST FUNDAMENTAL: Boa is embeddin JavaScript engine written in Rust; FFI pattern
// FFI = Foreign Function Interface; calling external code from Rust or vice versa
// Context manages JavaScript execution state, heap, etc.
use boa_engine::{Context, JsResult, JsValue, NativeFunction, Source, JsString};

// Import Boa object builder
// RUST FUNDAMENTAL: Builder pattern for complex object initialization
// ObjectInitializer fluent API: init().property(name, value).build()
use boa_engine::object::ObjectInitializer;

// Import property attributes (used below)
// RUST FUNDAMENTAL: Attributes control property behavior (readonly, writable, enumerable, etc.)
use boa_engine::property::Attribute;

// Import Boa garbage collection traits
// RUST FUNDAMENTAL: Trace and Finalize are garbage collection traits in Boa
// Rust's ownership model + GC marker traits = sound memory management for managed objects
use boa_gc::{Trace, Finalize, empty_trace};

// Import collections for storing nodes
use std::collections::BTreeMap;

// Import Rc for shared references
// RUST FUNDAMENTAL: Rc<T> enables multiple ownership in single thread
// Each clone increments reference count; dropped when count reaches 0
use std::rc::Rc;

// Import RefCell for interior mutability
// RUST FUNDAMENTAL: RefCell provides runtime (not compile-time) borrow checking
// Allows &self to return &mut T safely; panics if borrowed twice mutably
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
