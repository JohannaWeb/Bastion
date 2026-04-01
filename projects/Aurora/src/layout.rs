use crate::css::{DisplayMode, EdgeSizes, Margin, MarginValue, StyleMap, TextAlign};
use crate::style::{StyleTree, StyledNode};
use std::fmt::{self, Display, Formatter};

#[allow(dead_code)]
const DEFAULT_VIEWPORT_WIDTH: f32 = 1200.0;
const BLOCK_VERTICAL_PADDING: f32 = 8.0;
const TEXT_CHAR_WIDTH: f32 = 8.0;
const TEXT_LINE_HEIGHT: f32 = 14.0;
const INLINE_BOX_HEIGHT: f32 = 14.0;

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutTree {
    root: LayoutBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    kind: LayoutKind,
    rect: Rect,
    styles: StyleMap,
    margin: Margin,
    border: EdgeSizes,
    padding: EdgeSizes,
    children: Vec<LayoutBox>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LayoutKind {
    Viewport,
    Block { tag_name: String },
    Inline { tag_name: String },
    Image {
        alt: Option<String>,
        src: Option<String>,
        display_mode: DisplayMode,
    },
    Text { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutTree {
    #[allow(dead_code)]
    pub fn from_style_tree(style_tree: &StyleTree) -> Self {
        Self::from_style_tree_with_viewport_width(style_tree, DEFAULT_VIEWPORT_WIDTH)
    }

    pub fn from_style_tree_with_viewport_width(
        style_tree: &StyleTree,
        viewport_width: f32,
    ) -> Self {
        let root = LayoutBox::layout_root(style_tree.root(), viewport_width)
            .expect("style tree root must produce a viewport");
        Self { root }
    }

    pub fn root(&self) -> &LayoutBox {
        &self.root
    }
}

impl LayoutBox {
    fn layout_root(node: &StyledNode, viewport_width: f32) -> Option<Self> {
        let mut root = Self::from_styled_node(node, 0.0, 0.0, viewport_width)?;
        root.rect.width = viewport_width;
        Some(root)
    }

    fn from_styled_node(node: &StyledNode, x: f32, y: f32, available_width: f32) -> Option<Self> {
        if node.tag_name() == Some("style".to_string()) || node.tag_name() == Some("script".to_string()) {
            return None;
        }

        match node.tag_name() {
            None if node.text().is_none() => Some(Self::layout_container(
                LayoutKind::Viewport,
                node.styles().clone(),
                Margin::zero(),
                EdgeSizes::zero(),
                EdgeSizes::zero(),
                node.children(),
                x,
                y,
                available_width,
            )),
            Some(tag_name) => Self::from_element(&tag_name, node, x, y, available_width),
            None => Some(Self::layout_text(&node.text().unwrap_or_default(), node.styles().clone(), x, y)),
        }
    }

    fn from_element(
        tag_name: &str,
        node: &StyledNode,
        x: f32,
        y: f32,
        available_width: f32,
    ) -> Option<Self> {
        let styles = node.styles().clone();
        match styles.display_mode() {
            DisplayMode::None => None,
            mode if tag_name == "img" => Some(Self::layout_image(
                node,
                styles,
                node.styles().margin(),
                node.styles().border_width(),
                node.styles().padding(),
                x,
                y,
                available_width,
                mode,
            )),
            DisplayMode::Block => Some(Self::layout_container(
                LayoutKind::Block {
                    tag_name: tag_name.to_string(),
                },
                styles,
                node.styles().margin(),
                node.styles().border_width(),
                node.styles().padding(),
                node.children(),
                x,
                y,
                available_width,
            )),
            DisplayMode::Inline => Some(Self::layout_inline(
                tag_name,
                styles,
                node.styles().margin(),
                node.styles().border_width(),
                node.styles().padding(),
                node.children(),
                x,
                y,
                available_width,
            )),
        }
    }

    fn layout_image(
        node: &StyledNode,
        styles: StyleMap,
        margin: Margin,
        border: EdgeSizes,
        padding: EdgeSizes,
        x: f32,
        y: f32,
        available_width: f32,
        display_mode: DisplayMode,
    ) -> Self {
        let rect_x = x + margin.left.to_px();
        let rect_y = y + margin.top;
        let available_rect_width = (available_width - margin.horizontal()).max(0.0);
        let width_hint = node
            .attribute("width").as_deref()
            .and_then(parse_html_length_px)
            .unwrap_or(available_width);
        let height_hint = node
            .attribute("height").as_deref()
            .and_then(parse_html_length_px)
            .unwrap_or(96.0);
        let content_width = clamp_content_width(&styles, width_hint, available_rect_width);
        let content_height = clamp_content_height(&styles, height_hint);

        Self {
            kind: LayoutKind::Image {
                src: node.attribute("src").map(|s| s.to_string()),
                alt: node.attribute("alt").map(|s| s.to_string()),
                display_mode,
            },
            rect: Rect {
                x: rect_x,
                y: rect_y,
                width: (content_width + padding.horizontal() + border.horizontal()).min(available_rect_width),
                height: content_height + padding.vertical() + border.vertical(),
            },
            styles,
            margin,
            border,
            padding,
            children: Vec::new(),
        }
    }

    fn layout_container(
        kind: LayoutKind,
        styles: StyleMap,
        margin: Margin,
        border: EdgeSizes,
        padding: EdgeSizes,
        children: &[StyledNode],
        x: f32,
        y: f32,
        available_width: f32,
    ) -> Self {
        let mut rect_x = x + margin.left.to_px();
        let rect_y = y + margin.top;
        let available_rect_width = (available_width - margin.horizontal()).max(0.0);
        let default_content_width =
            (available_rect_width - padding.horizontal() - border.horizontal()).max(0.0);
        let content_width = clamp_content_width(&styles, default_content_width, default_content_width);

        // Handle margin: auto for block centering
        if let (MarginValue::Auto, MarginValue::Auto) = (margin.left, margin.right) {
            let total_box_width = content_width + padding.horizontal() + border.horizontal();
            let free_space = (available_width - total_box_width).max(0.0);
            rect_x = x + free_space / 2.0;
        }

        let rect_width =
            (content_width + padding.horizontal() + border.horizontal()).min(available_rect_width);
        let content_x = rect_x + border.left + padding.left;
        let content_y = rect_y + border.top + padding.top;
        let mut layout_children = Vec::new();
        let mut cursor_y = content_y;
        let mut previous_block_bottom_margin: f32 = 0.0;
        let mut previous_was_block = false;
        let mut inline_group: Vec<&StyledNode> = Vec::new();

        let flush_inline_group = |inline_group: &mut Vec<&StyledNode>,
                                  layout_children: &mut Vec<LayoutBox>,
                                  cursor_y: &mut f32,
                                  content_x: f32,
                                  _content_y: f32,
                                  content_width: f32| {
            if inline_group.is_empty() {
                return;
            }

            if let Some(anon_inline) = Self::layout_inline_sequence(
                inline_group,
                content_x,
                *cursor_y,
                content_width,
            ) {
                *cursor_y += anon_inline.total_height();
                layout_children.push(anon_inline);
            }
            inline_group.clear();
        };

        for child in children {
            let child_is_block = child
                .tag_name()
                .map(|_| child.styles().display_mode() == DisplayMode::Block)
                .unwrap_or(false);

            if child_is_block {
                flush_inline_group(
                    &mut inline_group,
                    &mut layout_children,
                    &mut cursor_y,
                    content_x,
                    content_y,
                    content_width,
                );

                let child_margin = child.styles().margin();
                let collapse_overlap = if previous_was_block {
                    previous_block_bottom_margin.min(child_margin.top)
                } else {
                    0.0
                };

                if let Some(mut layout_child) =
                    Self::from_styled_node(child, content_x, cursor_y - collapse_overlap, content_width)
                {
                    let alignment = styles.text_align();
                    if alignment != TextAlign::Left {
                        let child_width = layout_child.total_width();
                        let offset = match alignment {
                            TextAlign::Center => (content_width - child_width) / 2.0,
                            TextAlign::Right => content_width - child_width,
                            TextAlign::Left => 0.0,
                        };
                        if offset > 0.0 {
                            layout_child.rect.x += offset;
                        }
                    }

                    cursor_y += layout_child.total_height();
                    previous_block_bottom_margin = layout_child.margin.bottom;
                    previous_was_block = true;
                    layout_children.push(layout_child);
                }
            } else {
                inline_group.push(child);
                previous_was_block = false;
            }
        }

        flush_inline_group(
            &mut inline_group,
            &mut layout_children,
            &mut cursor_y,
            content_x,
            content_y,
            content_width,
        );

        let content_height = cursor_y - content_y;
        let inner_height = if layout_children.is_empty() {
            BLOCK_VERTICAL_PADDING
        } else {
            content_height + BLOCK_VERTICAL_PADDING
        };
        let resolved_content_height = clamp_content_height(&styles, inner_height);

        Self {
            kind,
            rect: Rect {
                x: rect_x,
                y: rect_y,
                width: rect_width,
                height: border.top + padding.top + resolved_content_height + padding.bottom + border.bottom,
            },
            styles,
            margin,
            border,
            padding,
            children: layout_children,
        }
    }

    fn layout_inline(
        tag_name: &str,
        styles: StyleMap,
        margin: Margin,
        border: EdgeSizes,
        padding: EdgeSizes,
        children: &[StyledNode],
        x: f32,
        y: f32,
        available_width: f32,
    ) -> Self {
        let rect_x = x + margin.left.to_px();
        let rect_y = y + margin.top;
        let available_rect_width = (available_width - margin.horizontal()).max(0.0);
        let default_content_width = (available_rect_width - padding.horizontal() - border.horizontal()).max(TEXT_CHAR_WIDTH);
        let content_width = clamp_content_width(&styles, default_content_width, default_content_width);
        let max_rect_width = (content_width + padding.horizontal() + border.horizontal()).min(available_rect_width);
        let content_x = rect_x + border.left + padding.left;
        let content_y = rect_y + border.top + padding.top;
        
        let mut layout_children = Vec::new();
        let mut line_x = content_x;
        let mut line_y = content_y;
        let mut line_height: f32 = 0.0;
        let mut max_line_width: f32 = 0.0;

        for child in children {
            if let Some(text) = child.text() {
                let fragments = Self::layout_text_fragments(
                    &text,
                    child.styles().clone(),
                    content_x,
                    content_width,
                    &mut line_x,
                    &mut line_y,
                    &mut line_height,
                    &mut max_line_width,
                );
                layout_children.extend(fragments);
                continue;
            }

            let remaining_width = (content_width - (line_x - content_x)).max(TEXT_CHAR_WIDTH);
            if let Some(mut layout_child) = Self::from_styled_node(child, line_x, line_y, remaining_width) {
                if line_x > content_x && layout_child.total_width() > remaining_width {
                    max_line_width = max_line_width.max(line_x - content_x);
                    line_y += line_height.max(TEXT_LINE_HEIGHT);
                    line_x = content_x;
                    line_height = 0.0;

                    if let Some(reflowed_child) = Self::from_styled_node(child, line_x, line_y, content_width) {
                        layout_child = reflowed_child;
                    }
                }

                line_x += layout_child.total_width();
                line_height = line_height.max(layout_child.total_height());
                max_line_width = max_line_width.max(line_x - content_x);
                layout_children.push(layout_child);
            }
        }

        let content_used_width = if layout_children.is_empty() {
            content_width.min(120.0)
        } else {
            max_line_width.max((line_x - content_x).min(content_width))
        };
        let total_content_height = if layout_children.is_empty() {
            INLINE_BOX_HEIGHT
        } else {
            (line_y - content_y) + line_height.max(INLINE_BOX_HEIGHT)
        };
        let resolved_content_height = clamp_content_height(&styles, total_content_height);

        let alignment = styles.text_align();
        if alignment != TextAlign::Left {
            let mut line_start = 0;
            while line_start < layout_children.len() {
                let line_y_val = layout_children[line_start].rect.y;
                let mut line_end = line_start + 1;
                while line_end < layout_children.len() && layout_children[line_end].rect.y == line_y_val {
                    line_end += 1;
                }

                let row_width: f32 = layout_children[line_start..line_end].iter().map(|b| b.total_width()).sum();
                let offset = match alignment {
                    TextAlign::Center => (content_width - row_width) / 2.0,
                    TextAlign::Right => content_width - row_width,
                    TextAlign::Left => 0.0,
                };

                if offset > 0.0 {
                    for b in &mut layout_children[line_start..line_end] {
                        b.rect.x += offset;
                    }
                }
                line_start = line_end;
            }
        }

        Self {
            kind: LayoutKind::Inline { tag_name: tag_name.to_string() },
            rect: Rect {
                x: rect_x,
                y: rect_y,
                width: (content_used_width + padding.horizontal() + border.horizontal()).min(max_rect_width),
                height: resolved_content_height + padding.vertical() + border.vertical(),
            },
            styles,
            margin,
            border,
            padding,
            children: layout_children,
        }
    }

    fn layout_inline_sequence(
        children: &[&StyledNode],
        x: f32,
        y: f32,
        available_width: f32,
    ) -> Option<Self> {
        if children.is_empty() {
            return None;
        }

        let mut layout_children = Vec::new();
        let mut line_x = x;
        let mut line_y = y;
        let mut line_height: f32 = 0.0;
        let mut max_line_width: f32 = 0.0;

        for child in children {
            if let Some(text) = child.text() {
                let fragments = Self::layout_text_fragments(
                    &text,
                    child.styles().clone(),
                    x,
                    available_width,
                    &mut line_x,
                    &mut line_y,
                    &mut line_height,
                    &mut max_line_width,
                );
                layout_children.extend(fragments);
                continue;
            }

            let remaining_width = (available_width - (line_x - x)).max(TEXT_CHAR_WIDTH);
            if let Some(mut layout_child) = Self::from_styled_node(child, line_x, line_y, remaining_width) {
                if line_x > x && layout_child.total_width() > remaining_width {
                    max_line_width = max_line_width.max(line_x - x);
                    line_y += line_height.max(TEXT_LINE_HEIGHT);
                    line_x = x;
                    line_height = 0.0;

                    if let Some(reflowed_child) = Self::from_styled_node(child, line_x, line_y, available_width) {
                        layout_child = reflowed_child;
                    }
                }

                line_x += layout_child.total_width();
                line_height = line_height.max(layout_child.total_height());
                max_line_width = max_line_width.max(line_x - x);
                layout_children.push(layout_child);
            }
        }

        let content_used_width = if layout_children.is_empty() {
            available_width.min(120.0)
        } else {
            max_line_width.max((line_x - x).min(available_width))
        };
        let total_content_height = if layout_children.is_empty() {
            INLINE_BOX_HEIGHT
        } else {
            (line_y - y) + line_height.max(INLINE_BOX_HEIGHT)
        };

        Some(Self {
            kind: LayoutKind::Block { tag_name: "anonymous-inline".to_string() },
            rect: Rect { x, y, width: content_used_width, height: total_content_height },
            styles: StyleMap::default(),
            margin: Margin::zero(),
            border: EdgeSizes::zero(),
            padding: EdgeSizes::zero(),
            children: layout_children,
        })
    }

    fn layout_text(text: &str, styles: StyleMap, x: f32, y: f32) -> Self {
        let char_width = char_width_from_styles(&styles);
        let line_height = line_height_from_styles(&styles);

        Self {
            kind: LayoutKind::Text { text: text.to_string() },
            rect: Rect {
                x,
                y,
                width: text.chars().count() as f32 * char_width,
                height: line_height,
            },
            styles,
            margin: Margin::zero(),
            border: EdgeSizes::zero(),
            padding: EdgeSizes::zero(),
            children: Vec::new(),
        }
    }

    fn decode_entities(text: &str) -> String {
        text.replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&apos;", "'")
            .replace("&copy;", "©")
            .replace("&reg;", "®")
            .replace("&trade;", "™")
            .replace("&bull;", "•")
            .replace("&middot;", "·")
            .replace("&ndash;", "–")
            .replace("&mdash;", "—")
    }

    fn layout_text_fragments(
        text: &str,
        styles: StyleMap,
        x: f32,
        available_width: f32,
        line_x: &mut f32,
        line_y: &mut f32,
        line_height: &mut f32,
        max_line_width: &mut f32,
    ) -> Vec<Self> {
        let mut fragments = Vec::new();
        let text = Self::decode_entities(text);

        // Split on whitespace but preserve word boundaries with trailing spaces
        let mut words_with_spaces = Vec::new();
        let mut current_word = String::new();
        let mut in_whitespace = false;

        for ch in text.chars() {
            if ch.is_whitespace() {
                if !current_word.is_empty() {
                    words_with_spaces.push(current_word.clone());
                    current_word.clear();
                }
                words_with_spaces.push(" ".to_string()); // Preserve spaces
                in_whitespace = true;
            } else {
                if in_whitespace && !words_with_spaces.is_empty() {
                    // Combine space with next word
                    if let Some(last) = words_with_spaces.last_mut() {
                        if last == " " {
                            current_word = last.clone();
                            words_with_spaces.pop();
                        }
                    }
                }
                current_word.push(ch);
                in_whitespace = false;
            }
        }

        if !current_word.is_empty() {
            words_with_spaces.push(current_word);
        }

        if words_with_spaces.is_empty() {
            return fragments;
        }

        let char_width = char_width_from_styles(&styles);
        let base_line_height = line_height_from_styles(&styles);
        let mut current_line = String::new();

        for word in words_with_spaces {
            let candidate = format!("{}{}", current_line, word);
            let candidate_width = candidate.chars().count() as f32 * char_width;
            let used_width = *line_x - x;
            let remaining_width = (available_width - used_width).max(char_width);

            // Only wrap if we have content on the line and the new word won't fit
            if !current_line.is_empty() && candidate_width > remaining_width && word.trim() != "" {
                // On line break, trim trailing spaces but keep the word
                let line_to_render = current_line.trim_end().to_string();
                if !line_to_render.is_empty() {
                    let fragment = Self::layout_text(&line_to_render, styles.clone(), *line_x, *line_y);
                    *line_x += fragment.rect.width;
                    *max_line_width = max_line_width.max(*line_x - x);
                    fragments.push(fragment);
                }

                *line_y += (*line_height).max(base_line_height);
                *line_x = x;
                *line_height = 0.0;
                current_line = word.clone();
            } else {
                current_line = candidate;
            }
        }

        // Render final line, preserving spaces (don't trim)
        if !current_line.is_empty() {
            let last_width = current_line.chars().count() as f32 * char_width;
            if *line_x - x + last_width > available_width && *line_x > x {
                *line_y += (*line_height).max(base_line_height);
                *line_x = x;
                *line_height = 0.0;
            }
            let fragment = Self::layout_text(&current_line, styles.clone(), *line_x, *line_y);
            *line_x += fragment.rect.width;
            *max_line_width = max_line_width.max(*line_x - x);
            fragments.push(fragment);
        }

        fragments
    }

    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        match &self.kind {
            LayoutKind::Viewport => {
                writeln!(f, "{indent}viewport {}", self.rect)?;
            }
            LayoutKind::Block { tag_name } => {
                writeln!(
                    f,
                    "{indent}block<{tag_name}> {} {}",
                    format_styles(&self.styles),
                    self.rect
                )?;
            }
            LayoutKind::Inline { tag_name } => {
                writeln!(
                    f,
                    "{indent}inline<{tag_name}> {} {}",
                    format_styles(&self.styles),
                    self.rect
                )?;
            }
            LayoutKind::Image { alt, src, display_mode } => {
                let kind = if *display_mode == DisplayMode::Inline {
                    "inline"
                } else {
                    "block"
                };
                writeln!(
                    f,
                    "{indent}{kind}<img alt={:?} src={:?}> {} {}",
                    alt,
                    src,
                    format_styles(&self.styles),
                    self.rect
                )?;
            }
            LayoutKind::Text { text } => {
                writeln!(f, "{indent}text(\"{text}\") {}", self.rect)?;
            }
        }

        for child in &self.children {
            child.fmt_with_indent(f, depth + 1)?;
        }

        Ok(())
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn total_width(&self) -> f32 {
        self.margin.left.to_px() + self.rect.width + self.margin.right.to_px()
    }

    pub fn total_height(&self) -> f32 {
        self.margin.top + self.rect.height + self.margin.bottom
    }

    #[allow(dead_code)]
    pub fn padding(&self) -> EdgeSizes {
        self.padding
    }

    #[allow(dead_code)]
    pub fn content_rect(&self) -> Rect {
        Rect {
            x: self.rect.x + self.border.left + self.padding.left,
            y: self.rect.y + self.border.top + self.padding.top,
            width: (self.rect.width - self.border.horizontal() - self.padding.horizontal()).max(0.0),
            height: (self.rect.height - self.border.vertical() - self.padding.vertical()).max(0.0),
        }
    }

    pub fn padding_rect(&self) -> Rect {
        Rect {
            x: self.rect.x + self.border.left,
            y: self.rect.y + self.border.top,
            width: (self.rect.width - self.border.horizontal()).max(0.0),
            height: (self.rect.height - self.border.vertical()).max(0.0),
        }
    }

    pub fn styles(&self) -> &StyleMap {
        &self.styles
    }

    pub fn children(&self) -> &[LayoutBox] {
        &self.children
    }

    pub fn tag_name(&self) -> Option<&str> {
        match &self.kind {
            LayoutKind::Block { tag_name } | LayoutKind::Inline { tag_name } => Some(tag_name),
            LayoutKind::Image { .. } => Some("img"),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match &self.kind {
            LayoutKind::Text { text } => Some(text),
            _ => None,
        }
    }

    pub fn is_viewport(&self) -> bool {
        matches!(self.kind, LayoutKind::Viewport)
    }

    pub fn image_alt(&self) -> Option<&str> {
        match &self.kind {
            LayoutKind::Image { alt, .. } => alt.as_deref(),
            _ => None,
        }
    }

    pub fn image_src(&self) -> Option<&str> {
        match &self.kind {
            LayoutKind::Image { src, .. } => src.as_deref(),
            _ => None,
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self.kind, LayoutKind::Image { .. })
    }
}

fn char_width_from_styles(styles: &StyleMap) -> f32 {
    // Default font-size assumed 16px → char_width = font_size / 2.0
    let base_width = styles.font_size_px().map(|s| s / 2.0).unwrap_or(TEXT_CHAR_WIDTH);

    // Apply font-weight multiplier
    if styles.font_weight() == "bold" || styles.font_weight() == "700" {
        base_width * 1.1
    } else {
        base_width
    }
}

fn line_height_from_styles(styles: &StyleMap) -> f32 {
    styles.line_height_px()
        .or_else(|| styles.font_size_px().map(|s| s * 1.2))
        .unwrap_or(TEXT_LINE_HEIGHT)
}

impl Display for LayoutTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.root.fmt_with_indent(f, 0)
    }
}

impl Display for Rect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[x: {:.0}, y: {:.0}, w: {:.0}, h: {:.0}]",
            self.x, self.y, self.width, self.height
        )
    }
}

fn format_styles(styles: &StyleMap) -> String {
    if styles.is_empty() {
        "{}".to_string()
    } else {
        format!("{styles}")
    }
}

fn clamp_content_width(styles: &StyleMap, candidate_width: f32, available_width: f32) -> f32 {
    let mut width = styles.width_px().unwrap_or(candidate_width);
    if let Some(min_width) = styles.min_width_px() {
        width = width.max(min_width);
    }
    if let Some(max_width) = styles.max_width_px() {
        width = width.min(max_width);
    }
    width.min(available_width).max(0.0)
}

fn clamp_content_height(styles: &StyleMap, candidate_height: f32) -> f32 {
    let mut height = styles.height_px().unwrap_or(candidate_height);
    if let Some(min_height) = styles.min_height_px() {
        height = height.max(min_height);
    }
    if let Some(max_height) = styles.max_height_px() {
        height = height.min(max_height);
    }
    height.max(0.0)
}

fn parse_html_length_px(value: &str) -> Option<f32> {
    value
        .strip_suffix("px")
        .unwrap_or(value)
        .parse::<f32>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::LayoutTree;
    use crate::css::Stylesheet;
    use crate::dom::Node;
    use crate::style::StyleTree;

    #[test]
    fn builds_layout_boxes_with_geometry() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("p", vec![Node::text("Hello")])],
        )]);
        let stylesheet = Stylesheet::parse("p { display: inline; color: blue; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree(&style_tree);
        let rendered = layout.to_string();

        assert!(rendered.contains("viewport [x: 0, y: 0, w: 800"));
        assert!(rendered.contains("block<body> {} [x: 0, y: 0, w: 800"));
        assert!(rendered.contains("inline<p> {color: blue, display: inline}"));
        assert!(rendered.contains("text(\"Hello\") [x: 0, y: 0, w: 40, h: 14]"));
    }

    #[test]
    fn stacks_block_children_vertically() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![
                Node::element("section", vec![Node::text("One")]),
                Node::element("section", vec![Node::text("Two")]),
            ],
        )]);
        let stylesheet = Stylesheet::parse("");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree(&style_tree);
        let rendered = layout.to_string();

        assert!(rendered.contains("block<section> {} [x: 0, y: 0, w: 800, h: 22]"));
        assert!(rendered.contains("block<section> {} [x: 0, y: 22, w: 800, h: 22]"));
    }

    #[test]
    fn wraps_inline_text_across_multiple_lines() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element(
                "p",
                vec![Node::text("alpha beta gamma delta epsilon zeta")],
            )],
        )]);
        let stylesheet = Stylesheet::parse("p { display: inline; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 96.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("inline<p> {display: inline}"));
        assert!(rendered.contains("text(\"alpha beta\") [x: 0, y: 0, w: 80, h: 14]"));
        assert!(rendered.contains("text(\"gamma delta\") [x: 0, y: 14, w: 88, h: 14]"));
        assert!(rendered.contains("text(\"epsilon zeta\") [x: 0, y: 28, w: 96, h: 14]"));
    }

    #[test]
    fn wraps_inline_children_when_the_row_fills() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element(
                "span",
                vec![
                    Node::element("em", vec![Node::text("hello")]),
                    Node::element("strong", vec![Node::text("world")]),
                ],
            )],
        )]);
        let stylesheet =
            Stylesheet::parse("span { display: inline; } em { display: inline; } strong { display: inline; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 72.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("inline<em> {display: inline} [x: 0, y: 0, w: 40, h: 14]"));
        assert!(rendered.contains("inline<strong> {display: inline}"));
    }

    #[test]
    fn applies_margin_and_padding_to_block_layout() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("section", vec![Node::text("Box")])],
        )]);
        let stylesheet = Stylesheet::parse(
            "section { margin: 10px 12px; padding: 4px 6px; }",
        );
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 200.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("block<section> {margin: 10px 12px, padding: 4px 6px} [x: 12, y: 10, w: 176, h: 30]"));
        assert!(rendered.contains("text(\"Box\") [x: 18, y: 14, w: 24, h: 14]"));
    }

    #[test]
    fn includes_border_width_in_box_geometry() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("section", vec![Node::text("Border")])],
        )]);
        let stylesheet =
            Stylesheet::parse("section { border: 4px solid ember; padding: 6px; width: 80px; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 220.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("block<section> {border: 4px solid ember, padding: 6px, width: 80px} [x: 0, y: 0, w: 100, h: 42]"));
        assert!(rendered.contains("text(\"Border\") [x: 10, y: 10, w: 48, h: 14]"));
    }

    #[test]
    fn applies_fixed_width_and_height_to_block_boxes() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("section", vec![Node::text("Sized")])],
        )]);
        let stylesheet = Stylesheet::parse(
            "section { width: 120px; height: 48px; padding: 4px; }",
        );
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 300.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("block<section> {height: 48px, padding: 4px, width: 120px} [x: 0, y: 0, w: 128, h: 56]"));
        assert!(rendered.contains("text(\"Sized\") [x: 4, y: 4, w: 40, h: 14]"));
    }

    #[test]
    fn constrains_inline_wrapping_with_fixed_width() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element(
                "p",
                vec![Node::text("one two three four five")],
            )],
        )]);
        let stylesheet = Stylesheet::parse("p { display: inline; width: 64px; padding: 4px; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);
        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 240.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("inline<p> {display: inline, padding: 4px, width: 64px}"));
        assert!(rendered.contains("text(\"one two\") [x: 4, y: 4, w: 56, h: 14]"));
        assert!(rendered.contains("text(\"three\")"));
        assert!(rendered.contains("text(\"four\")"));
        assert!(rendered.contains("text(\"five\")"));
    }

    #[test]
    fn aligns_inline_text_horizontally() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("p", vec![Node::text("Center")])],
        )]);
        let stylesheet = Stylesheet::parse("p { display: inline; text-align: center; width: 100px; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 200.0);
        let rendered = layout.to_string();

        // "Center" is 6 chars. 6 * 8.0 = 48.0 px.
        // alignment offset = (100 - 48) / 2 = 26.0
        assert!(rendered.contains("text(\"Center\") [x: 26"));
    }

    #[test]
    fn clamps_block_width_and_height_with_min_and_max() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![
                Node::element("section", vec![Node::text("Min")]),
                Node::element("article", vec![Node::text("Max")]),
            ],
        )]);
        let stylesheet = Stylesheet::parse(
            "section { width: 40px; min-width: 80px; height: 12px; min-height: 24px; padding: 4px; } article { width: 180px; max-width: 96px; height: 120px; max-height: 40px; padding: 4px; }",
        );
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 240.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("block<section> {height: 12px, min-height: 24px, min-width: 80px, padding: 4px, width: 40px} [x: 0, y: 0, w: 88, h: 32]"));
        assert!(rendered.contains("block<article> {height: 120px, max-height: 40px, max-width: 96px, padding: 4px, width: 180px} [x: 0, y: 32, w: 104, h: 48]"));
    }

    #[test]
    fn collapses_vertical_margins_between_block_siblings() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![
                Node::element("section", vec![Node::text("One")]),
                Node::element("section", vec![Node::text("Two")]),
            ],
        )]);
        let stylesheet =
            Stylesheet::parse("section { margin-top: 12px; margin-bottom: 18px; padding: 4px; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 240.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("block<section> {margin-bottom: 18px, margin-top: 12px, padding: 4px} [x: 0, y: 12, w: 240, h: 30]"));
        assert!(rendered.contains("block<section> {margin-bottom: 18px, margin-top: 12px, padding: 4px} [x: 0, y: 60, w: 240, h: 30]"));
    }

    #[test]
    fn clamps_inline_width_before_wrapping() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element(
                "p",
                vec![Node::text("one two three four five")],
            )],
        )]);
        let stylesheet = Stylesheet::parse(
            "p { display: inline; width: 140px; max-width: 64px; min-height: 60px; padding: 4px; }",
        );
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);
        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 240.0);
        let rendered = layout.to_string();

        assert!(rendered.contains("inline<p> {display: inline, max-width: 64px, min-height: 60px, padding: 4px, width: 140px}"));
        assert!(rendered.contains("text(\"one two\")"));
        assert!(rendered.contains("text(\"three\")"));
        assert!(rendered.contains("text(\"four\")"));
        assert!(rendered.contains("text(\"five\")"));
    }

    #[test]
    fn omits_nodes_with_display_none() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("p", vec![Node::text("Hidden")])],
        )]);
        let stylesheet = Stylesheet::parse("p { display: none; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree(&style_tree);
        let rendered = layout.to_string();

        assert!(!rendered.contains("<p>"));
        assert!(!rendered.contains("Hidden"));
    }

    #[test]
    fn includes_border_width_in_inline_box_geometry() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element("span", vec![Node::text("Hi")])],
        )]);
        let stylesheet =
            Stylesheet::parse("span { display: inline; border: 4px solid ember; padding: 2px; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 200.0);
        let rendered = layout.to_string();

        assert!(rendered.contains(
            "inline<span> {border: 4px solid ember, display: inline, padding: 2px} [x: 0, y: 0, w: 28, h: 26]"
        ));
        assert!(rendered.contains("text(\"Hi\") [x: 6, y: 6, w: 16, h: 14]"));
    }

    #[test]
    fn lays_out_images_with_attributes_as_replaced_boxes() {
        let dom = Node::document(vec![Node::element(
            "body",
            vec![Node::element_with_attributes(
                "img",
                [
                    ("alt".to_string(), "grumpy cat".to_string()),
                    ("src".to_string(), "cat.txt".to_string()),
                    ("width".to_string(), "120".to_string()),
                    ("height".to_string(), "80".to_string()),
                ]
                .into_iter()
                .collect(),
                Vec::new(),
            )],
        )]);
        let stylesheet =
            Stylesheet::parse("img { display: inline; padding: 4px; border: 2px solid ember; }");
        let style_tree = StyleTree::from_dom(&dom, &stylesheet);

        let layout = LayoutTree::from_style_tree_with_viewport_width(&style_tree, 240.0);
        let rendered = layout.to_string();

        assert!(rendered.contains(
            "inline<img alt=Some(\"grumpy cat\") src=Some(\"cat.txt\")> {border: 2px solid ember, display: inline, padding: 4px} [x: 0, y: 0, w: 132, h: 92]"
        ));
    }
}
