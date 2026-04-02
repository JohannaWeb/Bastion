use crate::css::{StyleMap, Stylesheet};
use crate::dom::{ElementNode, Node};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(Default, Clone)]
struct InheritedStyles {
    color: Option<String>,
    font_size: Option<String>,
    font_weight: Option<String>,
    line_height: Option<String>,
    visibility: Option<String>,
    text_decoration: Option<String>,
    font_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleTree {
    root: StyledNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledNode {
    pub node: crate::dom::NodePtr,
    pub styles: StyleMap,
    pub children: Vec<StyledNode>,
}

impl StyleTree {
    pub fn from_dom(document: &crate::dom::NodePtr, stylesheet: &Stylesheet) -> Self {
        Self {
            root: StyledNode::from_dom_node(std::rc::Rc::clone(document), stylesheet, &[], InheritedStyles::default(), &[]),
        }
    }

    pub fn root(&self) -> &StyledNode {
        &self.root
    }
}

impl StyledNode {
    pub fn styles(&self) -> &StyleMap {
        &self.styles
    }

    pub fn children(&self) -> &[StyledNode] {
        &self.children
    }

    pub fn tag_name(&self) -> Option<String> {
        let node = self.node.borrow();
        if let Node::Element(el) = &*node {
            Some(el.tag_name.clone())
        } else {
            None
        }
    }

    pub fn text(&self) -> Option<String> {
        let node = self.node.borrow();
        if let Node::Text(text) = &*node {
            Some(text.clone())
        } else {
            None
        }
    }

    pub fn attribute(&self, name: &str) -> Option<String> {
        match &*self.node.borrow() {
            Node::Element(el) => el.attributes.get(name).cloned(),
            _ => None,
        }
    }

    fn from_dom_node(
        node: crate::dom::NodePtr,
        stylesheet: &Stylesheet,
        element_ancestors: &[crate::css::ElementData],
        inherited: InheritedStyles,
        style_ancestors: &[&StyleMap],
    ) -> Self {
        let node_borrow = node.borrow();
        match &*node_borrow {
            Node::Document { children } => {
                let children_vec = children.clone();
                drop(node_borrow);
                Self {
                    node,
                    styles: StyleMap::default(),
                    children: children_vec
                        .into_iter()
                        .map(|child| {
                            Self::from_dom_node(
                                child,
                                stylesheet,
                                element_ancestors,
                                inherited.clone(),
                                style_ancestors,
                            )
                        })
                        .collect(),
                }
            },
            Node::Element(element) => {
                let current_data = crate::css::ElementData {
                    tag_name: element.tag_name.clone(),
                    attributes: element.attributes.clone(),
                };
                let mut styles = stylesheet.styles_for(&current_data, element_ancestors);
                
                // Resolve CSS variables before inheritance!
                styles.resolve_vars(style_ancestors);

                // Inherit typography traits...
                if styles.get("color").is_none() {
                    if let Some(color) = &inherited.color {
                        styles.set("color", color);
                    }
                }
                if styles.get("font-size").is_none() {
                    if let Some(font_size) = &inherited.font_size {
                        styles.set("font-size", font_size);
                    }
                }
                if styles.get("font-weight").is_none() {
                    if let Some(font_weight) = &inherited.font_weight {
                        styles.set("font-weight", font_weight);
                    }
                }
                if styles.get("line-height").is_none() {
                    if let Some(line_height) = &inherited.line_height {
                        styles.set("line-height", line_height);
                    }
                }
                if styles.get("visibility").is_none() {
                    if let Some(visibility) = &inherited.visibility {
                        styles.set("visibility", visibility);
                    }
                }
                if styles.get("text-decoration").is_none() {
                    if let Some(text_decoration) = &inherited.text_decoration {
                        styles.set("text-decoration", text_decoration);
                    }
                }

                let mut next_element_ancestors = element_ancestors.to_vec();
                next_element_ancestors.push(current_data);

                let next_inherited = InheritedStyles {
                    color: styles.get("color").map(ToOwned::to_owned),
                    font_size: styles.get("font-size").map(ToOwned::to_owned),
                    font_weight: styles.get("font-weight").map(ToOwned::to_owned),
                    line_height: styles.get("line-height").map(ToOwned::to_owned),
                    visibility: styles.get("visibility").map(ToOwned::to_owned),
                    text_decoration: styles.get("text-decoration").map(ToOwned::to_owned),
                    font_style: styles.get("font-style").map(ToOwned::to_owned),
                };

                // Create a clone of the children from the element to avoid double borrow
                let element_children = element.children.clone();
                drop(node_borrow);

                let mut node_to_return = Self {
                    node,
                    styles,
                    children: Vec::new(),
                };

                let mut next_style_ancestors = style_ancestors.to_vec();
                next_style_ancestors.push(&node_to_return.styles);

                node_to_return.children = element_children
                    .into_iter()
                    .map(|child| {
                        Self::from_dom_node(
                            child,
                            stylesheet,
                            &next_element_ancestors,
                            next_inherited.clone(),
                            &next_style_ancestors,
                        )
                    })
                    .collect();

                node_to_return
            }
            Node::Text(text) => {
                let mut styles = StyleMap::default();
                styles.set("display", "inline");

                if let Some(color) = &inherited.color {
                    styles.set("color", color);
                }
                if let Some(font_size) = &inherited.font_size {
                    styles.set("font-size", font_size);
                }
                if let Some(font_weight) = &inherited.font_weight {
                    styles.set("font-weight", font_weight);
                }
                if let Some(line_height) = &inherited.line_height {
                    styles.set("line-height", line_height);
                }
                if let Some(visibility) = &inherited.visibility {
                    styles.set("visibility", visibility);
                }
                if let Some(text_decoration) = &inherited.text_decoration {
                    styles.set("text-decoration", text_decoration);
                }
                if let Some(font_style) = &inherited.font_style {
                    styles.set("font-style", font_style);
                }

                Self {
                    node: node.clone(),
                    styles,
                    children: Vec::new(),
                }
            }
        }
    }

    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        let node_borrow = self.node.borrow();
        match &*node_borrow {
            Node::Document { .. } => writeln!(f, "{indent}#styled-document")?,
            Node::Element(el) => {
                writeln!(f, "{indent}<{}> {}", el.tag_name, self.styles)?
            }
            Node::Text(text) => writeln!(f, "{indent}\"{text}\" {}", self.styles)?,
        }
        drop(node_borrow);

        for child in &self.children {
            child.fmt_with_indent(f, depth + 1)?;
        }

        Ok(())
    }
}

impl Display for StyleTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.root.fmt_with_indent(f, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::StyleTree;
    use crate::css::Stylesheet;
    use crate::dom::{Node, NodePtr};
    use std::collections::BTreeMap;

    fn element(tag: &str, children: Vec<NodePtr>) -> NodePtr {
        Node::element(tag, children)
    }

    #[test]
    fn computes_descendant_matched_styles() {
        let mut section_attributes = BTreeMap::new();
        section_attributes.insert("class".to_string(), "hero".to_string());
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element_with_attributes(
                "section",
                section_attributes,
                vec![Node::element("p", vec![Node::text("Hello")])],
            )],
        )]);

        let stylesheet = Stylesheet::parse("section.hero p { color: gold; display: inline; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);
        let rendered = style_tree.to_string();

        assert!(rendered.contains("<p> {color: gold, display: inline}"));
        assert!(rendered.contains("\"Hello\" {color: gold, display: inline}"));
    }

    #[test]
    fn inherits_color_to_descendants() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("p", vec![Node::text("Inherited")])],
        )]);

        let stylesheet = Stylesheet::parse("body { color: slate; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);
        let rendered = style_tree.to_string();

        assert!(rendered.contains("<p> {color: slate}"));
        assert!(rendered.contains("\"Inherited\" {color: slate, display: inline}"));
    }

    #[test]
    fn inherits_typography_properties() {
        let dom = Node::document(vec![element(
            "body",
            vec![element("p", vec![Node::text("Text")])],
        )]);

        let stylesheet = Stylesheet::parse("body { font-size: 16px; font-weight: bold; line-height: 20px; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);
        let rendered = style_tree.to_string();

        assert!(rendered.contains("font-size: 16px"));
        assert!(rendered.contains("font-weight: bold"));
        assert!(rendered.contains("line-height: 20px"));
    }

    #[test]
    fn inherits_visibility() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("p", vec![Node::text("Text")])],
        )]);

        let stylesheet = Stylesheet::parse("body { visibility: hidden; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);
        let rendered = style_tree.to_string();

        assert!(rendered.contains("visibility: hidden"));
    }
}
