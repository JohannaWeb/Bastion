use crate::dom::{Node, NodePtr};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: BTreeMap<String, String>,
}

pub struct Stylesheet {
    pub rules: Vec<Rule>,
    pub variables: BTreeMap<String, String>,  // CSS custom properties (--name: value)
}

impl Stylesheet {
    pub fn merge(&mut self, other: Stylesheet) {
        self.rules.extend(other.rules);
        self.variables.extend(other.variables);
    }

    pub fn user_agent_stylesheet() -> Self {
        Self::parse(
            "a, abbr, b, bdo, big, br, cite, code, dfn, em, i, img, input, kbd, label, map, object, output, q, samp, select, small, span, strong, sub, sup, textarea, time, tt, var { display: inline; } \
             b, strong { font-weight: bold; color: accent; } \
             i, em { font-style: italic; color: rust; } \
             h1 { font-size: 32px; font-weight: bold; text-align: center; color: coal; } \
             h2 { font-size: 24px; font-weight: bold; color: ink; } \
             h3 { font-size: 18px; font-weight: bold; color: ink; } \
             li { display: block; margin: 4px 0; } \
             div, section, article { display: block; } \
             head, style, script, link, meta, title, noscript, template { display: none; }"
        )
    }

    pub fn parse(source: &str) -> Self {
        let mut rules = Vec::new();
        let mut variables = BTreeMap::new();

        // Strip @media, @keyframes, @font-face, etc. before splitting on '}'
        let stripped = strip_at_rules(source);

        for (source_order, chunk) in stripped.split('}').enumerate() {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }

            let Some((selector_part, declarations_part)) = chunk.split_once('{') else {
                continue;
            };

            let selector_part = selector_part.trim();
            let declarations = declarations_part
                .split(';')
                .filter_map(|declaration| {
                    let declaration = declaration.trim();
                    if declaration.is_empty() {
                        return None;
                    }

                    let (name, value) = declaration.split_once(':')?;
                    let name = name.trim().to_string();
                    // Strip !important before storing
                    let value = value.trim().trim_end_matches("!important").trim().to_string();

                    // Collect CSS custom properties from :root, *, and html selectors
                    if name.starts_with("--") && matches!(selector_part, ":root" | "*" | "html") {
                        variables.insert(name.clone(), value.clone());
                    }

                    Some(Declaration { name, value })
                })
                .collect::<Vec<_>>();

            if declarations.is_empty() {
                continue;
            }

            // Split on commas to handle comma-separated selectors
            for selector_str in selector_part.split(',') {
                let selector_str = selector_str.trim();
                if selector_str.is_empty() {
                    continue;
                }

                if let Some(selector) = Selector::parse(selector_str) {
                    rules.push(Rule {
                        selector,
                        declarations: declarations.clone(),
                        source_order,
                    });
                }
            }
        }

        Self { rules, variables }
    }

    pub fn from_dom(document: &NodePtr, base_url: Option<&str>, identity: &opus::domain::Identity) -> Self {
        let mut source = String::new();
        collect_styles(document, base_url, identity, &mut source);
        Self::parse(&source)
    }

    pub fn styles_for(&self, element: &ElementData, ancestors: &[ElementData]) -> StyleMap {
        let mut styles = StyleMap::default();
        let mut matching_rules = self
            .rules
            .iter()
            .filter(|rule| rule.selector.matches(element, ancestors))
            .collect::<Vec<_>>();

        matching_rules.sort_by_key(|rule| (rule.selector.specificity(), rule.source_order));

        for rule in matching_rules {
            for declaration in &rule.declarations {
                // Resolve CSS variables in the value
                let resolved_value = self.resolve_variables(&declaration.value);
                styles
                    .0
                    .insert(declaration.name.clone(), resolved_value);
            }
        }

        styles
    }

    /// Resolve CSS variables (var(--name)) in a value, including fallbacks
    pub fn resolve_variables(&self, value: &str) -> String {
        let mut result = value.to_string();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;  // Prevent infinite loops

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                break;
            }

            let start = match result.find("var(") {
                Some(idx) => idx,
                None => break,
            };

            // Find the matching closing paren, accounting for nested parens
            let mut paren_depth = 1;
            let mut end_pos = start + 4;
            for (i, ch) in result[start + 4..].chars().enumerate() {
                if ch == '(' {
                    paren_depth += 1;
                } else if ch == ')' {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        end_pos = start + 4 + i;
                        break;
                    }
                }
            }

            if paren_depth != 0 {
                break;  // Unmatched parens
            }

            let var_content = &result[start + 4..end_pos];

            // Split on first comma to separate variable name from fallback
            let (var_name, fallback) = if let Some(comma_idx) = var_content.find(',') {
                let name = var_content[..comma_idx].trim();
                let fb = var_content[comma_idx + 1..].trim();
                (name.to_string(), Some(fb.to_string()))
            } else {
                (var_content.trim().to_string(), None)
            };

            // Try to resolve the variable
            let replacement = if let Some(var_value) = self.variables.get(&var_name) {
                var_value.clone()
            } else if let Some(fb) = fallback {
                // Use fallback if variable not found
                fb
            } else {
                // No variable and no fallback, keep original
                break;
            };

            // Replace var(...) with the resolved value
            result.replace_range(start..end_pos + 1, &replacement);
        }

        result
    }
}

fn collect_styles(
    node_ptr: &NodePtr,
    base_url: Option<&str>,
    identity: &opus::domain::Identity,
    output: &mut String,
) {
    let node = node_ptr.borrow();
    match &*node {
        Node::Document { children } => {
            for child in children {
                collect_styles(child, base_url, identity, output);
            }
        }
        Node::Element(element) => {
            if element.tag_name == "style" {
                for child in &element.children {
                    let child_borrow = child.borrow();
                    if let Node::Text(text) = &*child_borrow {
                        output.push_str(text);
                        output.push('\n');
                    }
                }
            } else if element.tag_name == "link"
                && element.attributes.get("rel").map(String::as_str) == Some("stylesheet")
            {
                if let (Some(base), Some(href)) = (base_url, element.attributes.get("href")) {
                    if let Ok(url) = crate::fetch::resolve_relative_url(base, href) {
                        if let Ok(css) = crate::fetch::fetch_string(&url, identity) {
                            output.push_str(&css);
                            output.push('\n');
                        }
                    }
                }
            }

            for child in &element.children {
                collect_styles(child, base_url, identity, output);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
    pub source_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub parts: Vec<SimpleSelector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

pub type Specificity = (u8, u8, u8);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyleMap(BTreeMap<String, String>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeSizes {
    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarginValue {
    Px(f32),
    Auto,
}

impl MarginValue {
    pub fn to_px(self) -> f32 {
        match self {
            Self::Px(px) => px,
            Self::Auto => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin {
    pub top: f32,
    pub right: MarginValue,
    pub bottom: f32,
    pub left: MarginValue,
}

impl Margin {
    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: MarginValue::Px(0.0),
            bottom: 0.0,
            left: MarginValue::Px(0.0),
        }
    }

    pub fn horizontal(&self) -> f32 {
        self.left.to_px() + self.right.to_px()
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Block,
    Inline,
    InlineBlock,
    Flex,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    Center,
    FlexEnd,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl StyleMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn display_mode(&self) -> DisplayMode {
        match self.0.get("display").map(String::as_str) {
            Some("inline") | Some("inline-block") => DisplayMode::Inline,
            Some("flex") => DisplayMode::Flex,
            Some("none") => DisplayMode::None,
            _ => DisplayMode::Block,
        }
    }

    pub fn flex_direction(&self) -> FlexDirection {
        match self.0.get("flex-direction").map(String::as_str) {
            Some("column") => FlexDirection::Column,
            _ => FlexDirection::Row,
        }
    }

    pub fn justify_content(&self) -> JustifyContent {
        match self.0.get("justify-content").map(String::as_str) {
            Some("center") => JustifyContent::Center,
            Some("flex-end") => JustifyContent::FlexEnd,
            Some("space-between") => JustifyContent::SpaceBetween,
            Some("space-around") => JustifyContent::SpaceAround,
            _ => JustifyContent::FlexStart,
        }
    }

    pub fn align_items(&self) -> AlignItems {
        match self.0.get("align-items").map(String::as_str) {
            Some("flex-start") => AlignItems::FlexStart,
            Some("center") => AlignItems::Center,
            Some("flex-end") => AlignItems::FlexEnd,
            _ => AlignItems::Stretch,
        }
    }

    pub fn flex_wrap(&self) -> bool {
        matches!(self.0.get("flex-wrap").map(String::as_str), Some("wrap"))
    }

    pub fn gap_px(&self) -> f32 {
        self.get("column-gap")
            .and_then(parse_length_px)
            .or_else(|| self.get("gap").and_then(parse_length_px))
            .unwrap_or(0.0)
    }

    pub fn text_align(&self) -> TextAlign {
        match self.0.get("text-align").map(String::as_str) {
            Some("center") => TextAlign::Center,
            Some("right") => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }

    pub fn box_sizing(&self) -> BoxSizing {
        match self.0.get("box-sizing").map(String::as_str) {
            Some("border-box") => BoxSizing::BorderBox,
            _ => BoxSizing::ContentBox,
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.insert(name.into(), value.into());
    }

    pub fn margin(&self) -> Margin {
        let mut margin = parse_margin_shorthand(self.get("margin"));
        if let Some(top) = self.get("margin-top").and_then(parse_length_px) { margin.top = top; }
        if let Some(right) = self.get("margin-right") { margin.right = parse_margin_value(right); }
        if let Some(bottom) = self.get("margin-bottom").and_then(parse_length_px) { margin.bottom = bottom; }
        if let Some(left) = self.get("margin-left") { margin.left = parse_margin_value(left); }
        margin
    }

    pub fn padding(&self) -> EdgeSizes {
        self.edge_sizes("padding")
    }

    pub fn border_width(&self) -> EdgeSizes {
        let mut edges = parse_box_shorthand(self.get("border-width"));
        if edges == EdgeSizes::zero() {
            edges = parse_border_width_shorthand(self.get("border"));
        }
        edges.top = self.length_or("border-top-width", edges.top);
        edges.right = self.length_or("border-right-width", edges.right);
        edges.bottom = self.length_or("border-bottom-width", edges.bottom);
        edges.left = self.length_or("border-left-width", edges.left);
        edges
    }

    pub fn background_color(&self) -> Option<&str> {
        self.get("background-color").or_else(|| self.get("background"))
    }

    pub fn border_color(&self) -> Option<&str> {
        self.get("border-color")
            .or_else(|| parse_border_color_shorthand(self.get("border")))
    }

    pub fn width_px(&self) -> Option<f32> {
        self.get("width").and_then(parse_length_px)
    }

    pub fn height_px(&self) -> Option<f32> {
        self.get("height").and_then(parse_length_px)
    }

    pub fn min_width_px(&self) -> Option<f32> {
        self.get("min-width").and_then(parse_length_px)
    }

    pub fn max_width_px(&self) -> Option<f32> {
        self.get("max-width").and_then(parse_length_px)
    }

    pub fn min_height_px(&self) -> Option<f32> {
        self.get("min-height").and_then(parse_length_px)
    }

    pub fn max_height_px(&self) -> Option<f32> {
        self.get("max-height").and_then(parse_length_px)
    }

    pub fn font_size_px(&self) -> Option<f32> {
        self.get("font-size").and_then(parse_length_px)
    }

    /// Resolve width with support for %, rem, em, vw units.
    pub fn width_resolved(&self, available_width: f32, font_size: f32, root_font_size: f32, viewport_width: f32) -> Option<f32> {
        let raw = self.get("width")?;
        if raw == "auto" { return None; }
        parse_length_value(raw).map(|lv| lv.to_px(available_width, font_size, root_font_size, viewport_width, 0.0))
    }

    /// Resolve height with support for %, rem, em, vh units.
    pub fn height_resolved(&self, available_height: f32, font_size: f32, root_font_size: f32, viewport_height: f32) -> Option<f32> {
        let raw = self.get("height")?;
        if raw == "auto" || raw.contains("calc(") { return None; }
        parse_length_value(raw).map(|lv| lv.to_px(available_height, font_size, root_font_size, 0.0, viewport_height))
    }

    /// Resolve font-size with support for rem, em, % units.
    /// parent_font_size is used for em/% resolution.
    pub fn font_size_resolved(&self, parent_font_size: f32, root_font_size: f32) -> Option<f32> {
        let raw = self.get("font-size")?;
        parse_length_value(raw).map(|lv| lv.to_px(parent_font_size, parent_font_size, root_font_size, 0.0, 0.0))
    }

    pub fn font_weight(&self) -> &str {
        self.get("font-weight").unwrap_or("normal")
    }

    pub fn is_bold(&self) -> bool {
        let weight = self.font_weight();
        weight == "bold" || weight == "700" || weight == "bolder"
    }

    pub fn font_style(&self) -> &str {
        self.get("font-style").unwrap_or("normal")
    }

    pub fn is_italic(&self) -> bool {
        let style = self.font_style();
        style == "italic" || style == "oblique"
    }

    pub fn line_height_px(&self) -> Option<f32> {
        self.get("line-height").and_then(parse_length_px)
    }

    pub fn text_decoration(&self) -> Option<&str> {
        self.get("text-decoration")
    }

    pub fn opacity(&self) -> f32 {
        self.get("opacity")
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    }

    pub fn visibility(&self) -> &str {
        self.get("visibility").unwrap_or("visible")
    }

    pub fn resolve_vars(&mut self, ancestors: &[&StyleMap]) {
        let mut resolved = BTreeMap::new();
        for (name, value) in &self.0 {
            if value.contains("var(") {
                if let Some(new_value) = self.resolve_single_value(value, ancestors) {
                    resolved.insert(name.clone(), new_value);
                }
            }
        }
        for (name, value) in resolved {
            self.0.insert(name, value);
        }
    }

    fn resolve_single_value(&self, value: &str, ancestors: &[&StyleMap]) -> Option<String> {
        if !value.contains("var(") {
            return Some(value.to_string());
        }

        let mut result = String::new();
        let mut last_end = 0;
        
        // Very basic var() parser: find "var(--name)"
        // Note: Real CSS allows fallback: var(--name, fallback)
        while let Some(start) = value[last_end..].find("var(") {
            let start = last_end + start;
            result.push_str(&value[last_end..start]);
            
            let var_content_start = start + 4;
            if let Some(end) = value[var_content_start..].find(')') {
                let end = var_content_start + end;
                let var_expr = value[var_content_start..end].trim();
                
                // Handle fallback: var(--name, fallback)
                let (var_name, _fallback) = if let Some((name, fall)) = var_expr.split_once(',') {
                    (name.trim(), Some(fall.trim()))
                } else {
                    (var_expr, None)
                };

                if let Some(val) = self.lookup_variable(var_name, ancestors) {
                    result.push_str(&val);
                } else {
                    // If no value and no fallback, keep the var() expr or empty?
                    // Standard says it should be 'invalid at computed value time'.
                    // We'll leave it as is for now.
                    result.push_str(&value[start..end+1]);
                }
                last_end = end + 1;
            } else {
                break;
            }
        }
        result.push_str(&value[last_end..]);
        Some(result)
    }

    fn lookup_variable(&self, name: &str, ancestors: &[&StyleMap]) -> Option<String> {
        // Look in current map
        if let Some(val) = self.0.get(name) {
            return Some(val.clone());
        }
        // Look in ancestors (closest first)
        for ancestor in ancestors.iter().rev() {
            if let Some(val) = ancestor.0.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    fn edge_sizes(&self, prefix: &str) -> EdgeSizes {
        let mut edges = parse_box_shorthand(self.get(prefix));
        edges.top = self.length_or(format!("{prefix}-top").as_str(), edges.top);
        edges.right = self.length_or(format!("{prefix}-right").as_str(), edges.right);
        edges.bottom = self.length_or(format!("{prefix}-bottom").as_str(), edges.bottom);
        edges.left = self.length_or(format!("{prefix}-left").as_str(), edges.left);
        edges
    }

    fn length_or(&self, property: &str, fallback: f32) -> f32 {
        self.get(property)
            .and_then(parse_length_px)
            .unwrap_or(fallback)
    }
}

impl Selector {
    fn parse(source: &str) -> Option<Self> {
        let parts = source
            .split_whitespace()
            .map(SimpleSelector::parse)
            .collect::<Option<Vec<_>>>()?;

        if parts.is_empty() {
            return None;
        }

        Some(Self { parts })
    }

    pub fn matches(&self, element: &ElementData, ancestors: &[ElementData]) -> bool {
        let Some((last, previous)) = self.parts.split_last() else {
            return false;
        };

        if !last.matches_data(element) {
            return false;
        }

        let mut search_index = ancestors.len();
        for selector in previous.iter().rev() {
            let mut matched_index = None;
            while search_index > 0 {
                search_index -= 1;
                if selector.matches_data(&ancestors[search_index]) {
                    matched_index = Some(search_index);
                    break;
                }
            }

            if matched_index.is_none() {
                return false;
            }
        }

        true
    }

    pub fn specificity(&self) -> Specificity {
        self.parts.iter().fold((0, 0, 0), |acc, part| {
            let p = part.specificity();
            (acc.0 + p.0, acc.1 + p.1, acc.2 + p.2)
        })
    }
}

impl SimpleSelector {
    fn parse(source: &str) -> Option<Self> {
        // Strip pseudo-selectors (:hover, ::before) and attribute selectors ([type="text"])
        // instead of rejecting the whole rule — we just ignore those qualifiers
        let source = strip_pseudo_suffix(source);
        // Universal selector
        let source = source.trim_start_matches('*');

        let mut tag_name = String::new();
        let mut id = None;
        let mut class_names = Vec::new();
        let chars = source.chars().collect::<Vec<_>>();
        let mut index = 0;

        while index < chars.len() {
            match chars[index] {
                '#' => {
                    index += 1;
                    let start = index;
                    while index < chars.len() && is_identifier_char(chars[index]) {
                        index += 1;
                    }
                    if start == index || id.is_some() {
                        return None;
                    }
                    id = Some(chars[start..index].iter().collect());
                }
                '.' => {
                    index += 1;
                    let start = index;
                    while index < chars.len() && is_identifier_char(chars[index]) {
                        index += 1;
                    }
                    if start == index {
                        return None;
                    }
                    class_names.push(chars[start..index].iter().collect());
                }
                ch if is_identifier_char(ch) => {
                    if !tag_name.is_empty() {
                        return None;
                    }
                    let start = index;
                    while index < chars.len() && is_identifier_char(chars[index]) {
                        index += 1;
                    }
                    tag_name = chars[start..index].iter().collect();
                }
                _ => return None,
            }
        }

        if tag_name.is_empty() && id.is_none() && class_names.is_empty() {
            return None;
        }

        Some(Self {
            tag_name: if tag_name.is_empty() { None } else { Some(tag_name) },
            id,
            class_names,
        })
    }

    pub fn matches_data(&self, element: &ElementData) -> bool {
        if let Some(tag_name) = &self.tag_name {
            if &element.tag_name != tag_name {
                return false;
            }
        }

        if let Some(id) = &self.id {
            if element.attributes.get("id") != Some(id) {
                return false;
            }
        }

        if !self.class_names.is_empty() {
            let classes = element.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
            let element_classes: Vec<&str> = classes.split_whitespace().collect();
            if !self.class_names.iter().all(|cn| element_classes.contains(&cn.as_str())) {
                return false;
            }
        }

        true
    }

    fn specificity(&self) -> (u8, u8, u8) {
        (
            u8::from(self.id.is_some()),
            self.class_names.len() as u8,
            u8::from(self.tag_name.is_some()),
        )
    }
}

impl Display for Stylesheet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.rules.is_empty() {
            return writeln!(f, "(empty)");
        }

        for rule in &self.rules {
            write!(f, "{} ", rule.selector)?;
            write!(f, "{{")?;
            for (index, declaration) in rule.declarations.iter().enumerate() {
                if index > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}: {};", declaration.name, declaration.value)?;
            }
            writeln!(f, " }}")?;
        }

        Ok(())
    }
}

impl Display for Selector {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (index, part) in self.parts.iter().enumerate() {
            if index > 0 {
                write!(f, " ")?;
            }
            write!(f, "{part}")?;
        }
        Ok(())
    }
}

impl Display for SimpleSelector {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(tag_name) = &self.tag_name {
            write!(f, "{tag_name}")?;
        }
        if let Some(id) = &self.id {
            write!(f, "#{id}")?;
        }
        for cn in &self.class_names {
            write!(f, ".{cn}")?;
        }
        Ok(())
    }
}

impl Display for StyleMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;

        for (index, (name, value)) in self.0.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{name}: {value}")?;
        }

        write!(f, "}}")
    }
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

/// Strip pseudo-selectors (`:hover`, `::before`) and attribute selectors (`[type="text"]`)
/// from a simple selector string. Instead of rejecting rules that use these, we accept
/// the base selector and ignore the qualifiers.
fn strip_pseudo_suffix(s: &str) -> &str {
    let mut paren_depth: i32 = 0;
    let mut byte_pos = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            ':' | '[' if paren_depth == 0 => return &s[..i],
            _ => {}
        }
        byte_pos = i + ch.len_utf8();
    }
    &s[..byte_pos]
}

/// Remove @-rule blocks (@media, @keyframes, @font-face, etc.) from CSS source.
/// These contain nested braces that break the simple `split('}')` parser.
fn strip_at_rules(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '@' {
            // Consume the at-keyword and optional whitespace
            let mut found_brace = false;
            for c in chars.by_ref() {
                if c == '{' {
                    found_brace = true;
                    break;
                } else if c == ';' {
                    // Simple at-rule (e.g. @import, @charset) — no block, just skip
                    break;
                }
            }
            if found_brace {
                // Skip the entire block, tracking nested braces
                let mut depth = 1usize;
                for c in chars.by_ref() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn parse_margin_shorthand(value: Option<&str>) -> Margin {
    let Some(value) = value else {
        return Margin::zero();
    };

    let parts = value
        .split_whitespace()
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [all] => {
            let val = parse_margin_value(all);
            Margin {
                top: val.to_px(),
                right: val,
                bottom: val.to_px(),
                left: val,
            }
        }
        [vertical, horizontal] => {
            let v = parse_margin_value(vertical);
            let h = parse_margin_value(horizontal);
            Margin {
                top: v.to_px(),
                right: h,
                bottom: v.to_px(),
                left: h,
            }
        }
        [top, horizontal, bottom] => {
            let t = parse_margin_value(top);
            let h = parse_margin_value(horizontal);
            let b = parse_margin_value(bottom);
            Margin {
                top: t.to_px(),
                right: h,
                bottom: b.to_px(),
                left: h,
            }
        }
        [top, right, bottom, left] => {
            let t = parse_margin_value(top);
            let r = parse_margin_value(right);
            let b = parse_margin_value(bottom);
            let l = parse_margin_value(left);
            Margin {
                top: t.to_px(),
                right: r,
                bottom: b.to_px(),
                left: l,
            }
        }
        _ => Margin::zero(),
    }
}

fn parse_margin_value(value: &str) -> MarginValue {
    if value == "auto" {
        MarginValue::Auto
    } else {
        MarginValue::Px(parse_length_px(value).unwrap_or(0.0))
    }
}

fn parse_box_shorthand(value: Option<&str>) -> EdgeSizes {
    let Some(value) = value else {
        return EdgeSizes::zero();
    };

    let parts = value
        .split_whitespace()
        .filter_map(parse_length_px)
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [all] => EdgeSizes {
            top: *all,
            right: *all,
            bottom: *all,
            left: *all,
        },
        [vertical, horizontal] => EdgeSizes {
            top: *vertical,
            right: *horizontal,
            bottom: *vertical,
            left: *horizontal,
        },
        [top, horizontal, bottom] => EdgeSizes {
            top: *top,
            right: *horizontal,
            bottom: *bottom,
            left: *horizontal,
        },
        [top, right, bottom, left] => EdgeSizes {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        },
        _ => EdgeSizes::zero(),
    }
}

fn parse_length_px(value: &str) -> Option<f32> {
    let value = value.trim();
    if value == "0" {
        return Some(0.0);
    }

    value.strip_suffix("px")?.trim().parse::<f32>().ok()
}

/// A length value that may be in various CSS units.
#[derive(Debug, Clone, Copy)]
pub enum LengthValue {
    Px(f32),
    Percent(f32),
    Rem(f32),
    Em(f32),
    Vh(f32),
    Vw(f32),
}

impl LengthValue {
    /// Resolve to pixels given context.
    pub fn to_px(self, available: f32, font_size: f32, root_font_size: f32, viewport_width: f32, viewport_height: f32) -> f32 {
        match self {
            LengthValue::Px(v) => v,
            LengthValue::Percent(v) => available * v / 100.0,
            LengthValue::Rem(v) => root_font_size * v,
            LengthValue::Em(v) => font_size * v,
            LengthValue::Vw(v) => viewport_width * v / 100.0,
            LengthValue::Vh(v) => viewport_height * v / 100.0,
        }
    }
}

pub fn parse_length_value(value: &str) -> Option<LengthValue> {
    let value = value.trim();
    if value == "0" {
        return Some(LengthValue::Px(0.0));
    }
    if let Some(v) = value.strip_suffix("px") {
        return v.trim().parse::<f32>().ok().map(LengthValue::Px);
    }
    if let Some(v) = value.strip_suffix('%') {
        return v.trim().parse::<f32>().ok().map(LengthValue::Percent);
    }
    if let Some(v) = value.strip_suffix("rem") {
        return v.trim().parse::<f32>().ok().map(LengthValue::Rem);
    }
    if let Some(v) = value.strip_suffix("em") {
        return v.trim().parse::<f32>().ok().map(LengthValue::Em);
    }
    if let Some(v) = value.strip_suffix("vw") {
        return v.trim().parse::<f32>().ok().map(LengthValue::Vw);
    }
    if let Some(v) = value.strip_suffix("vh") {
        return v.trim().parse::<f32>().ok().map(LengthValue::Vh);
    }
    None
}

fn parse_border_width_shorthand(value: Option<&str>) -> EdgeSizes {
    let Some(value) = value else {
        return EdgeSizes::zero();
    };

    let widths = value
        .split_whitespace()
        .filter_map(parse_length_px)
        .collect::<Vec<_>>();

    match widths.first().copied() {
        Some(width) => EdgeSizes {
            top: width,
            right: width,
            bottom: width,
            left: width,
        },
        None => EdgeSizes::zero(),
    }
}

fn parse_border_color_shorthand(value: Option<&str>) -> Option<&str> {
    let value = value?;
    value
        .split_whitespace()
        .find(|part| parse_length_px(part).is_none() && *part != "solid")
}
