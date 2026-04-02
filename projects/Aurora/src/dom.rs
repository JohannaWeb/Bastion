use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use std::rc::Rc;
use std::cell::RefCell;

pub type NodePtr = Rc<RefCell<Node>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Document { children: Vec<NodePtr> },
    Element(ElementNode),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementNode {
    pub tag_name: String,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<NodePtr>,
}

impl Node {
    pub fn document(children: Vec<NodePtr>) -> NodePtr {
        Rc::new(RefCell::new(Self::Document { children }))
    }

    pub fn element_with_attributes(
        tag_name: impl Into<String>,
        attributes: BTreeMap<String, String>,
        children: Vec<NodePtr>,
    ) -> NodePtr {
        Rc::new(RefCell::new(Self::Element(ElementNode {
            tag_name: tag_name.into(),
            attributes,
            children,
        })))
    }

    pub fn element(tag_name: impl Into<String>, children: Vec<NodePtr>) -> NodePtr {
        Self::element_with_attributes(tag_name, BTreeMap::new(), children)
    }

    pub fn text(value: impl Into<String>) -> NodePtr {
        Rc::new(RefCell::new(Self::Text(value.into())))
    }

    pub fn find_node_by_id(&self, id: &str) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        if self.find_node_by_id_recursive(id, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    fn find_node_by_id_recursive(&self, id: &str, path: &mut Vec<usize>) -> bool {
        match self {
            Node::Document { children } => {
                for (i, child) in children.iter().enumerate() {
                    path.push(i);
                    if child.borrow().find_node_by_id_recursive(id, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            Node::Element(element) => {
                if element.attributes.get("id").map(|v| v == id).unwrap_or(false) {
                    return true;
                }
                for (i, child) in element.children.iter().enumerate() {
                    path.push(i);
                    if child.borrow().find_node_by_id_recursive(id, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            Node::Text(_) => {}
        }
        false
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<NodePtr> {
        match self {
            Node::Document { children } => children.get(index).cloned(),
            Node::Element(element) => element.children.get(index).cloned(),
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

    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        match self {
            Node::Document { children } => {
                writeln!(f, "{indent}#document")?;
                for child in children {
                    child.borrow().fmt_with_indent(f, depth + 1)?;
                }
                Ok(())
            }
            Node::Element(element) => {
                write!(f, "{indent}<{}", element.tag_name)?;
                for (name, value) in &element.attributes {
                    write!(f, " {name}=\"{value}\"")?;
                }
                writeln!(f, ">")?;
                for child in &element.children {
                    child.borrow().fmt_with_indent(f, depth + 1)?;
                }
                Ok(())
            }
            Node::Text(text) => writeln!(f, "{indent}\"{text}\""),
        }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}
