// Import DOM node types
use crate::dom::Node;
// Import BTreeMap for storing attributes in sorted order
use std::collections::BTreeMap;

// Enum representing different token types found in HTML
// RUST FUNDAMENTAL: Enums without associated data are memory-efficient; compiler optimizes discriminant
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    // Opening tag with tag name and attributes
    // RUST FUNDAMENTAL: Tuple variant Token::OpenTag(TagToken) wraps another struct
    // Match with: Token::OpenTag(tag) to destructure the TagToken
    OpenTag(TagToken),

    // Closing tag with tag name
    // RUST FUNDAMENTAL: Unit variant with associated data - variant can hold owned String
    CloseTag(String),

    // Text content between tags
    // RUST FUNDAMENTAL: String ownership - each token owns its text data
    // No references/borrowing here to avoid lifetime complications with token ownership
    Text(String),
}

// Struct holding data about an opening tag
#[derive(Debug, Clone, PartialEq, Eq)]
struct TagToken {
    // Name of the tag (e.g., "div", "p")
    tag_name: String,

    // Map of attribute names to their values
    // RUST FUNDAMENTAL: BTreeMap is preferred for attributes because:
    // 1. Ordered iteration (deterministic); 2. CSS selectors may iterate ordered
    attributes: BTreeMap<String, String>,
}

// Parser struct for converting HTML strings to DOM
// RUST FUNDAMENTAL: Generic lifetime parameter 'a - reference lives as long as source HTML
// Parser<'a> means: "I hold a reference borrowed for lifetime 'a"
pub struct Parser<'a> {
    // Pre-tokenized HTML as vector of tokens
    // RUST FUNDAMENTAL: Vec<Token> is heap-allocated; owns all tokens (not borrowed)
    // Tokenization happens upfront before parsing
    tokens: Vec<Token>,

    // Current position in token stream
    // RUST FUNDAMENTAL: usize is machine-word integer; used for indexing (overflow panics in debug)
    position: usize,

    // Reference to original HTML source (kept but not used)
    // RUST FUNDAMENTAL: &'a str is borrowed string slice - doesn't own data
    // Lifetime 'a ensures reference outlives Parser; Parser can't outlive source
    // #[allow(dead_code)] suppresses warning for intentionally unused field (kept for potential future use)
    #[allow(dead_code)]
    source: &'a str,
}

// Implementation of Parser methods
// RUST FUNDAMENTAL: impl<'a> Parser<'a> means: implement for Parser with any lifetime 'a
// Generic lifetime parameters work like generic types but track reference lifetimes
impl<'a> Parser<'a> {
    // Create a new parser from HTML source string
    // RUST FUNDAMENTAL: pub fn new(source: &'a str) takes reference with lifetime 'a
    // 'a is tied to Parser lifetime - Parser can't outlive source
    pub fn new(source: &'a str) -> Self {
        // Tokenize HTML string and create parser state
        Self {
            // Pre-tokenize HTML into token stream
            // RUST FUNDAMENTAL: tokenize() takes &str (borrowed), returns Vec<Token> (owned)
            // Tokenization is eager (happens upfront); alternative would be lazy iterators
            tokens: tokenize(source),

            // Start at beginning of token stream
            position: 0,

            // Keep reference to original source
            // RUST FUNDAMENTAL: source: &'a str ties lifetime 'a to this field
            // Ensures Parser doesn't outlive source string
            source,
        }
    }

    // Parse entire HTML document into DOM tree
    // RUST FUNDAMENTAL: &mut self means: need mutable borrow; parse_document modifies self.position
    pub fn parse_document(&mut self) -> crate::dom::NodePtr {
        // Initialize vector to collect top-level nodes
        // RUST FUNDAMENTAL: Vec::new() creates empty heap-allocated vector; grows dynamically
        let mut children = Vec::new();

        // Parse nodes until end of token stream
        // RUST FUNDAMENTAL: while loop with condition; while !self.is_eof() checks if more tokens exist
        // More concise than traditional for-loop with index management
        while !self.is_eof() {
            // Try to parse a node at current position
            // RUST FUNDAMENTAL: if let Some(node) = self.parse_node() unwraps Option in condition
            // None case is handled by else block; Some(value) is bound to node
            if let Some(node) = self.parse_node() {
                // Add parsed node to children vector
                // RUST FUNDAMENTAL: .push() adds to end; moves ownership of node into Vec
                // Vec owns all children nodes
                children.push(node);
            } else {
                // If no node found, advance position
                // RUST FUNDAMENTAL: &mut self allows modifying self.position
                self.position += 1;
            }
        }

        // Wrap children in document node
        // RUST FUNDAMENTAL: Function call consumes children Vec; moves into Node::document()
        Node::document(children)
    }

    // Parse a single node (text, element, or closing tag)
    // RUST FUNDAMENTAL: fn parse_node(&mut self) -> Option<NodePtr> returns Option
    // Caller uses match/if let to handle Some/None cases
    fn parse_node(&mut self) -> Option<crate::dom::NodePtr> {
        // Get current token without advancing
        // RUST FUNDAMENTAL: self.peek()? is try operator - returns None if peek() returns None
        // Equivalent to match self.peek() { Some(token) => token, None => return None }
        match self.peek()? {
            // If current token is text
            // RUST FUNDAMENTAL: Pattern match on Token enum variants
            // Each arm handles different variant; compiler ensures all covered
            Token::Text(text) => {
                // Clone text content
                // RUST FUNDAMENTAL: .clone() needed because peek() returns &Token (borrowed)
                // Can't move out of borrowed reference; must clone to get owned String
                let text = text.clone();

                // Advance to next token
                self.position += 1;

                // Return text node
                // RUST FUNDAMENTAL: Some(value) wraps value in Option; Option returned to caller
                Some(Node::text(text))
            }

            // If current token is opening tag
            Token::OpenTag(tag) => {
                // Clone tag data
                let tag = tag.clone();

                // Advance to next token
                self.position += 1;

                // Check if tag is void (self-closing)
                // RUST FUNDAMENTAL: Conditional execution of two different branches
                // Both return Option, satisfying return type
                if is_void_tag(&tag.tag_name) {
                    // Create element without children for void tags
                    // RUST FUNDAMENTAL: Vec::new() creates empty vector for void element children
                    Some(Node::element_with_attributes(
                        tag.tag_name,
                        tag.attributes,
                        Vec::new(),
                    ))
                } else {
                    // Parse element with children for normal tags
                    // RUST FUNDAMENTAL: Recursive call to parse_element
                    // Allows arbitrary nesting; stack grows with recursion depth
                    Some(self.parse_element(tag))
                }
            }

            // If current token is closing tag, skip it
            // RUST FUNDAMENTAL: _ is catch-all pattern; ignores value in CloseTag(String)
            // Returning None signals no node parsed at this position
            Token::CloseTag(_) => None,
        }
    }

    // Parse an element and its children until matching close tag
    fn parse_element(&mut self, tag: TagToken) -> crate::dom::NodePtr {
        // Initialize vector to collect child nodes
        let mut children = Vec::new();

        // Parse children until matching close tag or end
        while let Some(token) = self.peek() {
            // Match token type
            match token {
                // If matching close tag found, stop parsing children
                Token::CloseTag(close_tag) if close_tag == &tag.tag_name => {
                    // Advance past close tag
                    self.position += 1;
                    // Exit loop
                    break;
                }
                // If non-matching close tag, stop parsing (mismatched tags)
                Token::CloseTag(_) => break,
                // For other tokens, try to parse as child node
                _ => {
                    // Attempt to parse child node
                    if let Some(node) = self.parse_node() {
                        // Add child node to children vector
                        children.push(node);
                    }
                }
            }
        }

        // Create element with collected children
        Node::element_with_attributes(tag.tag_name, tag.attributes, children)
    }

    // Peek at current token without consuming it
    fn peek(&self) -> Option<&Token> {
        // Return reference to token at current position
        self.tokens.get(self.position)
    }

    // Check if we've reached end of token stream
    fn is_eof(&self) -> bool {
        // True if position is at or past token count
        self.position >= self.tokens.len()
    }
}

// Convert HTML source string into token stream
fn tokenize(source: &str) -> Vec<Token> {
    // Initialize result token vector
    let mut tokens = Vec::new();
    // Initialize text accumulation buffer
    let mut text_buffer = String::new();
    // Current byte offset in source
    let mut index = 0;

    // Process source string character by character
    while index < source.len() {
        // Get substring from current position to end
        let rest = &source[index..];
        // Get next character, break if none
        let Some(ch) = rest.chars().next() else {
            break;
        };

        // Check if character is tag start
        if ch == '<' {
            // If text buffer has content, emit it as text token
            if !text_buffer.is_empty() {
                // Collapse consecutive whitespace
                let collapsed = collapse_whitespace(&text_buffer);
                // Only emit if non-whitespace content
                if !collapsed.trim().is_empty() {
                    // Decode HTML entities and create text token
                    tokens.push(Token::Text(decode_entities(&collapsed)));
                }
            }
            // Clear text buffer for next segment
            text_buffer.clear();

            // Find end of tag (next '>'), skip if not found
            let Some(tag_end_offset) = find_tag_end(rest) else {
                // Not a tag, treat '<' as text
                text_buffer.push(ch);
                // Advance by character width (UTF-8 safe)
                index += ch.len_utf8();
                continue;
            };

            // Skip DOCTYPE declarations (not needed for DOM)
            if rest.starts_with("<!DOCTYPE") || rest.starts_with("<!doctype") {
                // Skip entire DOCTYPE tag
                index += tag_end_offset + 1;
                continue;
            }

            // Skip HTML comments
            if rest.starts_with("<!--") {
                // Find comment end marker
                if let Some(comment_end) = rest.find("-->") {
                    // Skip past comment end
                    index += comment_end + 3;
                } else {
                    // Unterminated comment, skip to end
                    index += 4;
                }
                continue;
            }

            // Extract tag content between < and >
            let tag = rest[1..tag_end_offset].trim();
            // Advance past closing >
            index += tag_end_offset + 1;

            // Check if tag is closing tag (starts with /)
            if let Some(stripped) = tag.strip_prefix('/') {
                // Create close tag token with tag name
                tokens.push(Token::CloseTag(stripped.trim().to_string()));
            } else if !tag.is_empty() {
                // Parse opening tag into tag name and attributes
                let open_tag = parse_open_tag(tag);
                // Check if tag needs raw text parsing (script, style)
                let raw_text_tag = is_raw_text_tag(&open_tag.tag_name);
                // Keep tag name for later use
                let tag_name = open_tag.tag_name.clone();
                // Emit opening tag token
                tokens.push(Token::OpenTag(open_tag));

                // Special handling for raw text tags
                if raw_text_tag {
                    // Build closing tag string to search for
                    let close_tag = format!("</{tag_name}>");
                    // Find position of closing tag
                    if let Some(close_offset) = source[index..].find(&close_tag) {
                        // Extract raw text between tags
                        let raw_text = decode_entities(&source[index..index + close_offset]);
                        // Emit non-empty text tokens
                        if !raw_text.trim().is_empty() {
                            tokens.push(Token::Text(raw_text.trim().to_string()));
                        }
                        // Emit closing tag token
                        tokens.push(Token::CloseTag(tag_name));
                        // Skip past raw text and closing tag
                        index += close_offset + close_tag.len();
                    } else {
                        // No closing tag found, emit remaining as text
                        let raw_text = decode_entities(&source[index..]);
                        if !raw_text.trim().is_empty() {
                            tokens.push(Token::Text(raw_text.trim().to_string()));
                        }
                        // Stop processing (reached end of file)
                        break;
                    }
                }
            }
        } else {
            // Non-tag character, add to text buffer
            text_buffer.push(ch);
            // Advance by character width (UTF-8 safe)
            index += ch.len_utf8();
        }
    }

    // Emit any remaining text in buffer
    if !text_buffer.is_empty() {
        // Collapse consecutive whitespace
        let collapsed = collapse_whitespace(&text_buffer);
        // Only emit if non-whitespace content
        if !collapsed.trim().is_empty() {
            // Decode entities and create text token
            tokens.push(Token::Text(decode_entities(&collapsed)));
        }
    }

    // Return token stream
    tokens
}

// Collapse multiple consecutive whitespace into single space
fn collapse_whitespace(input: &str) -> String {
    // Initialize result string
    let mut result = String::new();
    // Track if previous character was whitespace
    let mut last_was_whitespace = false;

    // Iterate through each character in input
    for ch in input.chars() {
        // Check if character is whitespace
        if ch.is_whitespace() {
            // Only emit space if previous wasn't whitespace
            if !last_was_whitespace {
                // Add single space
                result.push(' ');
                // Mark that we just added whitespace
                last_was_whitespace = true;
            }
        } else {
            // Non-whitespace character, add it directly
            result.push(ch);
            // Mark that last character wasn't whitespace
            last_was_whitespace = false;
        }
    }
    // Return collapsed string
    result
}

// Replace common HTML entities with their Unicode equivalents
fn decode_entities(input: &str) -> String {
    // Apply chain of replace operations for common entities
    input
        // Non-breaking space to regular space
        .replace("&nbsp;", " ")
        // Less than symbol
        .replace("&lt;", "<")
        // Greater than symbol
        .replace("&gt;", ">")
        // Ampersand
        .replace("&amp;", "&")
        // Double quote
        .replace("&quot;", "\"")
        // Single quote
        .replace("&apos;", "'")
        // Copyright symbol
        .replace("&copy;", "\u{00A9}")
        // Registered trademark symbol
        .replace("&reg;", "\u{00AE}")
        // Trademark symbol
        .replace("&trade;", "\u{2122}")
        // Bullet point
        .replace("&bull;", "\u{2022}")
        // Middle dot
        .replace("&middot;", "\u{00B7}")
        // En-dash
        .replace("&ndash;", "\u{2013}")
        // Em-dash
        .replace("&mdash;", "\u{2014}")
}

// Parse an opening tag into tag name and attributes
fn parse_open_tag(source: &str) -> TagToken {
    // Remove trailing slash and whitespace, convert to chars
    let mut chars = source.trim_end_matches('/').trim_end().chars().peekable();
    // Initialize tag name buffer
    let mut tag_name = String::new();

    // Extract tag name (everything until first whitespace)
    while let Some(ch) = chars.peek() {
        // Stop when we hit whitespace
        if ch.is_whitespace() {
            break;
        }
        // Append character to tag name
        tag_name.push(*ch);
        // Move to next character
        chars.next();
    }

    // Skip whitespace between tag name and attributes
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        // Move past whitespace character
        chars.next();
    }

    // Collect remaining characters into string
    let rest = chars.collect::<String>();
    // Create tag token with name and parsed attributes
    TagToken {
        tag_name,
        // Parse attribute string into key-value pairs
        attributes: parse_attributes(&rest),
    }
}

// Check if tag should be parsed as raw text (no nested tags)
fn is_raw_text_tag(tag_name: &str) -> bool {
    // Match script and style tags that preserve content literally
    matches!(tag_name, "script" | "style")
}

// Check if tag is self-closing (void) with no content
fn is_void_tag(tag_name: &str) -> bool {
    // Match HTML void elements that have no closing tag
    matches!(tag_name, "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr")
}

// Find the position of '>' that closes a tag (respecting quotes)
fn find_tag_end(source: &str) -> Option<usize> {
    // Convert source to vector of chars for indexing
    let chars: Vec<char> = source.chars().collect();
    // Current position in character vector
    let mut i = 0;
    // Track if we're inside quoted string and which quote type
    let mut quote_char: Option<char> = None;

    // Loop through all characters
    while i < chars.len() {
        // Match character and current quote state
        match (chars[i], quote_char) {
            // Found double quote while not quoted, start quoted section
            ('"', None) => quote_char = Some('"'),
            // Found matching double quote, end quoted section
            ('"', Some('"')) => quote_char = None,
            // Found single quote while not quoted, start quoted section
            ('\'', None) => quote_char = Some('\''),
            // Found matching single quote, end quoted section
            ('\'', Some('\'')) => quote_char = None,
            // Found '>' while not in quoted section, return position
            ('>', None) => return Some(i),
            // All other characters, do nothing
            _ => {}
        }
        // Move to next character
        i += 1;
    }

    // No closing '>' found
    None
}

// Parse attribute string into key-value pairs map
fn parse_attributes(source: &str) -> BTreeMap<String, String> {
    // Initialize result map for attributes
    let mut attributes = BTreeMap::new();
    // Convert source string to character vector
    let chars = source.chars().collect::<Vec<_>>();
    // Current position in character vector
    let mut index = 0;

    // Main parsing loop
    while index < chars.len() {
        // Skip leading whitespace before attribute name
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }

        // Stop if we've reached end of string
        if index >= chars.len() {
            break;
        }

        // Find start of attribute name
        let start = index;
        // Scan until whitespace or '=' (end of attribute name)
        while index < chars.len() && !chars[index].is_whitespace() && chars[index] != '=' {
            index += 1;
        }

        // Skip if we didn't read any name characters (shouldn't happen)
        if start == index {
            index += 1;
            continue;
        }

        // Extract attribute name from characters
        let name = chars[start..index].iter().collect::<String>();

        // Skip whitespace after attribute name
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }

        // Parse attribute value (may be absent)
        let value = if index < chars.len() && chars[index] == '=' {
            // Skip past '=' character
            index += 1;
            // Skip whitespace after '='
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }

            // Check if we reached end of string (empty value)
            if index >= chars.len() {
                String::new()
            } else if chars[index] == '"' || chars[index] == '\'' {
                // Quoted value
                // Remember which quote character opened the value
                let quote = chars[index];
                // Skip opening quote
                index += 1;
                // Mark start of value content
                let value_start = index;
                // Scan until matching closing quote
                while index < chars.len() && chars[index] != quote {
                    index += 1;
                }
                // Extract value content between quotes
                let value = chars[value_start..index].iter().collect::<String>();
                // Skip closing quote if present
                if index < chars.len() {
                    index += 1;
                }
                // Return parsed value
                value
            } else {
                // Unquoted value
                // Mark start of value content
                let value_start = index;
                // Scan until whitespace (end of value)
                while index < chars.len() && !chars[index].is_whitespace() {
                    index += 1;
                }
                // Extract value content
                chars[value_start..index].iter().collect::<String>()
            }
        } else {
            // No '=' found, this is a boolean attribute
            String::new()
        };

        // Store parsed attribute in map
        attributes.insert(name, value);
    }

    // Return completed attributes map
    attributes
}

// Test module for HTML parser (only compiled in test builds)
#[cfg(test)]
mod tests {
    // Import Parser for testing
    use super::Parser;
    // Import DOM node types for assertions
    use crate::dom::{Node, NodePtr};
    // Import BTreeMap for creating expected attributes
    use std::collections::BTreeMap;

    // Helper function to create element nodes for testing
    fn element(tag: &str, children: Vec<NodePtr>) -> NodePtr {
        // Create element with tag, no attributes, and given children
        Node::element_with_attributes(tag, BTreeMap::new(), children)
    }

    // Test that parser correctly builds DOM tree from nested HTML
    #[test]
    fn parses_nested_html_into_dom_tree() {
        let mut parser = Parser::new("<html><body><p>Hello</p><p>World</p></body></html>");
        let document = parser.parse_document();

        assert_eq!(
            document,
            Node::document(vec![element(
                "html",
                vec![element(
                    "body",
                    vec![
                        element("p", vec![Node::text("Hello")]),
                        element("p", vec![Node::text("World")]),
                    ],
                )],
            )])
        );
    }

    #[test]
    fn ignores_whitespace_only_text_nodes() {
        let mut parser = Parser::new("<div>\n  <p>Text</p>\n</div>");
        let document = parser.parse_document();

        assert_eq!(
            document,
            Node::document(vec![Node::element(
                "div",
                vec![Node::element("p", vec![Node::text("Text")])],
            )])
        );
    }

    #[test]
    fn keeps_script_contents_as_raw_text() {
        let mut parser = Parser::new("<script>if (a < b) { run(); }</script>");
        let document = parser.parse_document();

        assert_eq!(
            document,
            Node::document(vec![Node::element(
                "script",
                vec![Node::text("if (a < b) { run(); }")],
            )])
        );
    }

    #[test]
    fn keeps_mismatched_closing_tag_for_parent_recovery() {
        let mut parser = Parser::new("<div><span>t</div><p>x</p>");
        let document = parser.parse_document();

        assert_eq!(
            document,
            Node::document(vec![
                Node::element(
                    "div",
                    vec![Node::element("span", vec![Node::text("t")])],
                ),
                Node::element("p", vec![Node::text("x")]),
            ])
        );
    }

    #[test]
    fn treats_img_as_void_element() {
        let mut parser = Parser::new("<div><img src=\"cat.txt\" alt=\"sleepy cat\"><p>caption</p></div>");
        let document = parser.parse_document();

        let mut img_attributes = BTreeMap::new();
        img_attributes.insert("alt".to_string(), "sleepy cat".to_string());
        img_attributes.insert("src".to_string(), "cat.txt".to_string());

        assert_eq!(
            document,
            Node::document(vec![Node::element(
                "div",
                vec![
                    Node::element_with_attributes("img", img_attributes, Vec::new()),
                    Node::element("p", vec![Node::text("caption")]),
                ],
            )])
        );
    }

    #[test]
    fn preserves_tag_attributes() {
        let mut parser = Parser::new(
            r#"<div id="app" class="shell main"><p data-role=hero hidden>Hello</p></div>"#,
        );
        let document = parser.parse_document();

        let mut div_attributes = BTreeMap::new();
        div_attributes.insert("class".to_string(), "shell main".to_string());
        div_attributes.insert("id".to_string(), "app".to_string());

        let mut p_attributes = BTreeMap::new();
        p_attributes.insert("data-role".to_string(), "hero".to_string());
        p_attributes.insert("hidden".to_string(), String::new());

        assert_eq!(
            document,
            Node::document(vec![Node::element_with_attributes(
                "div",
                div_attributes,
                vec![Node::element_with_attributes(
                    "p",
                    p_attributes,
                    vec![Node::text("Hello")],
                )],
            )])
        );
    }

    #[test]
    fn handles_quoted_attributes_with_special_characters() {
        let mut parser = Parser::new(
            r#"<a href="http://example.com?foo=bar>baz" title="Text with 'quotes' > inside">Link</a>"#,
        );
        let document = parser.parse_document();

        let mut a_attributes = BTreeMap::new();
        a_attributes.insert("href".to_string(), "http://example.com?foo=bar>baz".to_string());
        a_attributes.insert("title".to_string(), "Text with 'quotes' > inside".to_string());

        assert_eq!(
            document,
            Node::document(vec![Node::element_with_attributes(
                "a",
                a_attributes,
                vec![Node::text("Link")],
            )])
        );
    }

    #[test]
    fn handles_json_in_data_attributes() {
        let mut parser = Parser::new(
            r#"<div data-config='{"key":"value","num":>0}'></div>"#,
        );
        let document = parser.parse_document();

        let mut div_attributes = BTreeMap::new();
        div_attributes.insert("data-config".to_string(), "{\"key\":\"value\",\"num\":>0}".to_string());

        assert_eq!(
            document,
            Node::document(vec![Node::element_with_attributes(
                "div",
                div_attributes,
                vec![],
            )])
        );
    }
}
