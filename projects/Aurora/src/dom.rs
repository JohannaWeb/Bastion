// Import BTreeMap for ordered attribute storage
use std::collections::BTreeMap;
// Import Display and Formatter traits for custom printing
use std::fmt::{self, Display, Formatter};

// Import Rc for shared reference counting pointers
use std::rc::Rc;
// Import RefCell for interior mutability (allow mutation through shared ref)
use std::cell::RefCell;

// Type alias: NodePtr wraps a Node in Rc<RefCell<>> for shared mutable access
// RUST FUNDAMENTAL: Type aliases (pub type) create new names for existing types; they're just aliases (not distinct types)
// NodePtr = Rc<RefCell<Node>>: allows multiple ownership (Rc) with interior mutability (RefCell)
pub type NodePtr = Rc<RefCell<Node>>;
// RUST FUNDAMENTAL: This pattern solves the problem: DOM trees need cyclic references (parent->child, child->parent)
// Normal Rust ownership is acyclic - can't express cycles without smart pointers
// Rc = Reference Counted; each clone increments counter; dropped when counter = 0
// RefCell = Runtime borrow checker; catches double-mutable borrow at runtime (panics), not compile-time

// Enum representing different types of DOM nodes
// RUST FUNDAMENTAL: Enums in Rust can hold data (variants are like union types + discriminant)
// Each variant has unique data; matched with pattern matching
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    // Document root node containing top-level children
    // RUST FUNDAMENTAL: Named struct variant { children: Vec<NodePtr> }
    // Access with: if let Node::Document { children } = node { ... }
    Document { children: Vec<NodePtr> },

    // Element node (tag) with attributes and child nodes
    // RUST FUNDAMENTAL: Tuple variant Node::Element(ElementNode) wraps another type
    // Access with: if let Node::Element(el) = node { ... }
    Element(ElementNode),

    // Text node containing raw string content
    // RUST FUNDAMENTAL: Unit-like variant Text(String) is essentially a wrapper
    Text(String),
}

// Struct representing an HTML element with tag name, attributes, and children
// RUST FUNDAMENTAL: pub struct makes all fields public by default; individual privacy not specified
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementNode {
    // HTML tag name (e.g., "div", "p", "span")
    // RUST FUNDAMENTAL: String is heap-allocated, owned UTF-8 text; unlike &str which is borrowed
    pub tag_name: String,

    // Map of attribute names to values (e.g., id, class, src)
    // RUST FUNDAMENTAL: BTreeMap<K, V> is ordered map (unlike unordered HashMap); useful for deterministic iteration
    pub attributes: BTreeMap<String, String>,

    // Vector of child node pointers
    // RUST FUNDAMENTAL: Vec<NodePtr> = Vec<Rc<RefCell<Node>>>; each child is shared/mutable via smart pointers
    pub children: Vec<NodePtr>,
}

// Implementation of Node factory methods
// RUST FUNDAMENTAL: impl Node blocks define methods/associated functions on the Node enum
// Methods (take self/&self/&mut self) vs associated functions (no self) - called with Type::func()
impl Node {
    // Create a document node wrapping top-level child nodes
    // RUST FUNDAMENTAL: Return type NodePtr = Rc<RefCell<Node>>; constructed with Rc::new(), RefCell::new()
    pub fn document(children: Vec<NodePtr>) -> NodePtr {
        // Wrap the Document variant in Rc<RefCell<>> for shared mutable access
        // RUST FUNDAMENTAL: Rc::new() wraps value; RefCell::new() inside adds runtime borrow checking
        // Can't use & borrowing because children Vec is moved; Rc allows sharing ownership
        Rc::new(RefCell::new(Self::Document { children }))
    }

    // Create an element node with tag name, attributes, and children
    // RUST FUNDAMENTAL: impl Into<String> accepts anything convertible to String (String, &str, etc.)
    // Generic parameter T: Into<String> provides more ergonomic API - users don't need .to_string()
    pub fn element_with_attributes(
        // Tag name convertible to String
        // RUST FUNDAMENTAL: impl Into<T> - trait bound; allows implicit conversion; compiler calls .into() automatically
        tag_name: impl Into<String>,

        // Map of attributes for this element
        attributes: BTreeMap<String, String>,

        // Vector of child nodes
        children: Vec<NodePtr>,
    ) -> NodePtr {
        // Wrap ElementNode in Rc<RefCell<>> for shared mutable access
        Rc::new(RefCell::new(Self::Element(ElementNode {
            // Convert tag name to String
            // RUST FUNDAMENTAL: .into() consumes self, returns different type based on context
            // Rust type inference determines Into<String> trait object based on expected type
            tag_name: tag_name.into(),

            // Store attributes map
            // RUST FUNDAMENTAL: Move semantics - attributes (BTreeMap) ownership moves into ElementNode
            // No copy happens; efficient zero-cost abstraction
            attributes,

            // Store child nodes
            children,
        })))
    }

    // Create an element node with tag name and children (no attributes)
    // RUST FUNDAMENTAL: Method reuse pattern - delegates to element_with_attributes with default
    pub fn element(tag_name: impl Into<String>, children: Vec<NodePtr>) -> NodePtr {
        // Delegate to element_with_attributes using empty attribute map
        // RUST FUNDAMENTAL: BTreeMap::new() creates empty map; move semantics mean efficient passing
        Self::element_with_attributes(tag_name, BTreeMap::new(), children)
    }

    // Create a text node containing a string
    pub fn text(value: impl Into<String>) -> NodePtr {
        // Wrap Text variant in Rc<RefCell<>> for shared mutable access
        // RUST FUNDAMENTAL: Rc<RefCell<>> needed because JavaScript might need to mutate text
        // Without RefCell, would need &mut reference, breaking shared ownership promise
        Rc::new(RefCell::new(Self::Text(value.into())))
    }

    // Find a node by its ID attribute and return the path to it
    pub fn find_node_by_id(&self, id: &str) -> Option<Vec<usize>> {
        // Create empty path vector to track indices
        let mut path = Vec::new();
        // Recursively search for node with matching ID
        if self.find_node_by_id_recursive(id, &mut path) {
            // Return path if found
            Some(path)
        } else {
            // Return None if not found
            None
        }
    }

    // Recursively search DOM tree for element with matching ID attribute
    fn find_node_by_id_recursive(&self, id: &str, path: &mut Vec<usize>) -> bool {
        // Match on current node type
        match self {
            // For document nodes, search all children
            Node::Document { children } => {
                // Iterate through children with their indices
                for (i, child) in children.iter().enumerate() {
                    // Record current index in path
                    path.push(i);
                    // Recursively check if child contains target ID
                    if child.borrow().find_node_by_id_recursive(id, path) {
                        // Return true if found
                        return true;
                    }
                    // Remove index from path (backtrack)
                    path.pop();
                }
            }
            // For element nodes, check ID then search children
            Node::Element(element) => {
                // Check if this element has matching ID attribute
                if element.attributes.get("id").map(|v| v == id).unwrap_or(false) {
                    // Return true if ID matches
                    return true;
                }
                // Iterate through children with their indices
                for (i, child) in element.children.iter().enumerate() {
                    // Record current index in path
                    path.push(i);
                    // Recursively check if child contains target ID
                    if child.borrow().find_node_by_id_recursive(id, path) {
                        // Return true if found
                        return true;
                    }
                    // Remove index from path (backtrack)
                    path.pop();
                }
            }
            // Text nodes have no attributes, skip them
            Node::Text(_) => {}
        }
        // Return false if target not found in this subtree
        false
    }

    // Get a child node by index (from Document or Element)
    pub fn get_child_mut(&mut self, index: usize) -> Option<NodePtr> {
        // Match on node type
        match self {
            // For document, return child at index if exists
            Node::Document { children } => children.get(index).cloned(),
            // For element, return child at index if exists
            Node::Element(element) => element.children.get(index).cloned(),
            // Text nodes have no children
            Node::Text(_) => None,
        }
    }

    /*
    pub fn get_node_at_path_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let mut current = self;
        for &index in path {
            match current {
                Node::Document { children } => {
                    current = &mut *children.get_mut(index)?.borrow_mut();
                }
                Node::Element(element) => {
                    current = &mut *element.children.get_mut(index)?.borrow_mut();
                }
                Node::Text(_) => return None,
            }
        }
        Some(current)
    }
    */

    // Helper function to format node with indentation for pretty-printing
    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> fmt::Result {
        // Create indentation string (2 spaces per depth level)
        let indent = "  ".repeat(depth);
        // Match on node type
        match self {
            // Document node: print label and all children indented
            Node::Document { children } => {
                // Write document label at current indentation
                writeln!(f, "{indent}#document")?;
                // Recursively format each child with increased indentation
                for child in children {
                    child.borrow().fmt_with_indent(f, depth + 1)?;
                }
                // Return ok result
                Ok(())
            }
            // Element node: print tag with attributes and children
            Node::Element(element) => {
                // Write opening tag name
                write!(f, "{indent}<{}", element.tag_name)?;
                // Write all attributes as name="value" pairs
                for (name, value) in &element.attributes {
                    write!(f, " {name}=\"{value}\"")?;
                }
                // Close opening tag
                writeln!(f, ">")?;
                // Recursively format each child with increased indentation
                for child in &element.children {
                    child.borrow().fmt_with_indent(f, depth + 1)?;
                }
                // Return ok result
                Ok(())
            }
            // Text node: print quoted text content
            Node::Text(text) => writeln!(f, "{indent}\"{text}\""),
        }
    }
}

// Trait implementation to convert Node to Display string
impl Display for Node {
    // Format node for display using indentation formatting
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Delegate to fmt_with_indent starting at depth 0
        self.fmt_with_indent(f, 0)
    }
}
