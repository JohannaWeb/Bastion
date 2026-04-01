use crate::dom::{Node, NodePtr};
use crate::dom::ElementNode;
use boa_engine::{Context, JsResult, JsValue, NativeFunction, Source, JsString};
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::object::builtins::JsFunction;
use boa_gc::{Trace, Finalize, empty_trace};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone)]
struct NodeHandle {
    inner: NodePtr,
}

unsafe impl Trace for NodeHandle {
    empty_trace!();
}

impl Finalize for NodeHandle {}

pub struct BoaRuntime {

    context: Context,
    #[allow(dead_code)]
    document: NodePtr,
}

impl BoaRuntime {
    pub fn new(document: NodePtr) -> Self {
        let mut context = Context::default();
        
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
                // For now, intercept network requests and return a mock synchronous response
                this.readyState = 4;
                this.status = 200;
                this.responseText = "{}"; // Send empty JSON object mock
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
        // context.create_builtin_function(JsString::from("fetch"), 1, fetch);
        context.register_global_property(JsString::from("console"), console, Attribute::all());

        // Document
        let doc_handle = NodeHandle { inner: document.clone() };
        let doc_obj = ObjectInitializer::new(&mut context)
            .function(NativeFunction::from_copy_closure_with_captures(
                |_this, args, captures, _context| {
                    let id = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                    if let Some(node) = find_by_id(&captures.inner, &id) {
                         Ok(JsValue::null())
                    } else {
                        Ok(JsValue::null())
                    }
                },
                doc_handle
            ), JsString::from("getElementById"), 1)
            .build();
        let _ = context.register_global_property(JsString::from("document"), doc_obj, Attribute::all());

        // Make window = globalObject to alias all globals correctly
        let global_obj = context.global_object().clone();
        let _ = context.register_global_property(JsString::from("window"), global_obj, Attribute::all());

        Self { context, document }
    }

    pub fn execute(&mut self, script: &str) -> JsResult<JsValue> {
        self.context.eval(Source::from_bytes(script))
    }
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

/*
fn create_js_node(node: NodePtr, context: &mut Context) -> JsValue {
    let handle = NodeHandle { inner: node };
    
    let getter = NativeFunction::from_copy_closure_with_captures(|_this, _args, captures, _context| {
        Ok(JsValue::from(JsString::from(captures.inner.borrow().to_string())))
    }, handle.clone());
    let getter_js = context.create_builtin_function(getter, 0, JsString::from("get_innerText"), &[]);

    let setter = NativeFunction::from_copy_closure_with_captures(|_this, args, captures, _context| {
        let new_text = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut n = captures.inner.borrow_mut();
        match &mut *n {
            Node::Element(el) => {
                el.children = vec![Node::text(new_text)];
            }
            _ => {}
        }
        Ok(JsValue::undefined())
    }, handle);
    let setter_js = context.create_builtin_function(JsString::from("set_innerText"), 1, setter, None);

    ObjectInitializer::new(context)
        .accessor(JsString::from("innerText"), 
            Some(getter_js),
            Some(setter_js),
            Attribute::all()
        )
        .build()
        .into()
}
*/
