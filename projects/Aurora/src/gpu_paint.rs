// Import layout box for rendering
use crate::layout::LayoutBox;
// Import Vello graphics primitives
use vello::kurbo::{RoundedRect, Affine, Rect as KRect};
// Import color and fill types
use peniko::{Color, Fill};
// Import Vello scene for GPU rendering
use vello::Scene;
// Import OnceLock for lazy static initialization (thread-safe)
use std::sync::OnceLock;
// Import Mutex for thread-safe interior mutability
use std::sync::Mutex;
// Import HashMap for color caching
use std::collections::HashMap;

// Global cache for dominant image colors (lazy-initialized, thread-safe)
// RUST FUNDAMENTAL: OnceLock<Mutex<T>> provides thread-safe lazy initialization
// T is computed once, then Mutex protects shared access
static IMAGE_COLOR_CACHE: OnceLock<Mutex<HashMap<String, [u8; 3]>>> = OnceLock::new();

// Get or initialize the image color cache
fn get_image_color_cache() -> &'static Mutex<HashMap<String, [u8; 3]>> {
    // RUST FUNDAMENTAL: 'static lifetime - valid for entire program duration
    // get_or_init() lazily initializes on first call, returns same reference thereafter
    IMAGE_COLOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// GPU painter for rendering layout boxes to Vello scene
// RUST FUNDAMENTAL: Unit struct (no data) - used for organizing methods via impl
pub struct GpuPainter;

// GpuPainter implementation
impl GpuPainter {
    // Paint layout box with full opacity
    pub fn paint(layout_box: &LayoutBox, scene: &mut Scene) {
        // Delegate to opacity-aware version with 1.0 opacity
        Self::paint_with_opacity(layout_box, scene, 1.0);
    }

    // Paint layout box with opacity blending through tree
    // RUST FUNDAMENTAL: parent_opacity f32 parameter shows how Rust passes primitives by value (Copy type)
    fn paint_with_opacity(layout_box: &LayoutBox, scene: &mut Scene, parent_opacity: f32) {
        // Get styles from layout box
        let styles = layout_box.styles();
        // Get opacity property (0.0 to 1.0)
        let opacity = styles.opacity();
        // Calculate effective opacity combining parent and current
        let effective_opacity = parent_opacity * opacity;

        // Skip rendering if completely transparent or hidden
        if effective_opacity < 0.01 || styles.visibility() == "hidden" {
            return;
        }

        // Get position and size
        let rect = layout_box.rect();

        // Render based on box type
        if layout_box.is_viewport() {
            // Viewport: fill entire background
            let bg_color = parse_color(styles.background_color().unwrap_or("white"));
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                bg_color,
                None,
                &KRect::new(rect.x as f64, rect.y as f64, (rect.x + rect.width) as f64, (rect.y + rect.height) as f64),
            );
        } else if layout_box.is_image() {
            // Image: special rendering
            paint_image(layout_box, scene);
        } else if let Some(text) = layout_box.text() {
            // Text: render with opacity
            paint_text_with_opacity(layout_box, text, scene, effective_opacity);
        } else {
            // Element: render with borders and background
            paint_element_with_opacity(layout_box, scene, effective_opacity);
        }

        // Recursively paint children with inherited opacity
        // RUST FUNDAMENTAL: for loop borrows children iterator
        for child in layout_box.children() {
            Self::paint_with_opacity(child, scene, effective_opacity);
        }
    }
}

// Paint an element (div, section, etc.) with background, borders, and shadow
fn paint_element_with_opacity(layout_box: &LayoutBox, scene: &mut Scene, opacity: f32) {
    // Get position/size rect
    let r = layout_box.rect();
    // Get computed styles
    let styles = layout_box.styles();

    // Get background color, trying multiple property names
    // RUST FUNDAMENTAL: .or_else() chains Option<T> handling; unwrap_or() provides default
    let bg_color_name = styles.get("background-color").or_else(|| styles.get("background")).unwrap_or("transparent");
    // Parse color string into Color object
    let mut bg_color = parse_color(bg_color_name);

    // Get border color, default to black
    let mut border_color = parse_color(styles.get("border-color").unwrap_or("black"));
    // Get border width
    let border = layout_box.styles().border_width();

    // Apply opacity to colors by multiplying alpha channel
    // RUST FUNDAMENTAL: .components[3] accesses array; *= modifies in-place
    bg_color.components[3] *= opacity;
    border_color.components[3] *= opacity;

    // Parse border-radius, default to 0.0
    let radius = if let Some(radius_str) = styles.get("border-radius") {
        // RUST FUNDAMENTAL: if let Some(x) = option unwraps Some variant
        // trim_end_matches() removes "px" suffix, parse() converts string to f32
        radius_str.trim_end_matches("px").parse::<f32>().unwrap_or(0.0) as f64
    } else {
        0.0
    };

    // Create rectangle in Vello's coordinate system
    let k_rect = KRect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
    // Create rounded rectangle with border radius
    let rounded_rect = RoundedRect::from_rect(k_rect, radius);

    // Paint drop shadow if present
    if let Some(shadow_str) = styles.get("box-shadow") {
        if shadow_str != "none" {
            // Create semi-transparent black shadow color with opacity
            let shadow_color = Color::from_rgba8(0, 0, 0, ((60.0 * opacity) as u8).min(255));
            // Shadow offset slightly down and right
            let shadow_rect = KRect::new(
                (r.x + 3.0) as f64,
                (r.y + 3.0) as f64,
                (r.x + r.width + 3.0) as f64,
                (r.y + r.height + 3.0) as f64,
            );
            // Fill shadow rectangle on scene
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                shadow_color,
                None,
                &RoundedRect::from_rect(shadow_rect, radius.max(2.0)),
            );
        }
    }

    // Paint background if not fully transparent
    if bg_color.components[3] > 0.0 {
        scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &rounded_rect);
    }

    if border.top > 0.0 {
        let stroke_width = border.top as f64;
        scene.stroke(&vello::kurbo::Stroke::new(stroke_width), Affine::IDENTITY, border_color, None, &rounded_rect);
    }
}

fn paint_text_with_opacity(layout_box: &LayoutBox, text: &str, scene: &mut Scene, opacity: f32) {
    let r = layout_box.rect();
    let styles = layout_box.styles();
    let mut text_color = parse_color(styles.get("color").unwrap_or("black"));
    text_color.components[3] *= opacity;

    let mut font_size = styles.font_size_px().filter(|&s| s > 0.0).unwrap_or(16.0);
    if let Ok(zoom_str) = std::env::var("AURORA_ZOOM") {
        if let Ok(zoom) = zoom_str.parse::<f32>() {
            font_size *= zoom;
        }
    }

    let text_align = styles.text_align();
    let text_decoration = styles.get("text-decoration").unwrap_or("none");

    let text_run = crate::font::layout_text_run(text, font_size);
    let text_width = text_run.width as f64;
    let offset_x = match text_align {
        crate::css::TextAlign::Center => (r.width as f64 - text_width).max(0.0) / 2.0,
        crate::css::TextAlign::Right => (r.width as f64 - text_width).max(0.0),
        crate::css::TextAlign::Left => 0.0,
    };

    let baseline_y = r.y as f64 + font_size as f64 * 0.75;
    let base_r = (text_color.components[0] * 255.0).round().clamp(0.0, 255.0) as u8;
    let base_g = (text_color.components[1] * 255.0).round().clamp(0.0, 255.0) as u8;
    let base_b = (text_color.components[2] * 255.0).round().clamp(0.0, 255.0) as u8;
    let base_a = (text_color.components[3] * 255.0).round().clamp(0.0, 255.0) as u8;

    for glyph in &text_run.glyphs {
        if let Some(raster) = crate::font::rasterize_glyph(glyph.ch, font_size) {
            let gx = r.x as f64 + offset_x + glyph.x as f64 + raster.x_offset as f64;
            let gy = baseline_y + glyph.y_offset as f64 + raster.y_offset as f64;

            for row in 0..raster.height {
                for col in 0..raster.width {
                    let idx = (row * raster.width + col) as usize;
                    let alpha = raster.bitmap.get(idx).copied().unwrap_or(0);
                    if alpha == 0 {
                        continue;
                    }

                    let coverage = alpha as f32 / 255.0;
                    let glyph_alpha = (base_a as f32 * coverage).round().clamp(0.0, 255.0) as u8;
                    let glyph_color = Color::from_rgba8(base_r, base_g, base_b, glyph_alpha);
                    let pixel_rect = KRect::new(
                        gx + col as f64,
                        gy + row as f64,
                        gx + col as f64 + 1.0,
                        gy + row as f64 + 1.0,
                    );
                    scene.fill(Fill::NonZero, Affine::IDENTITY, glyph_color, None, &pixel_rect);
                }
            }
        }
    }

    if text_decoration != "none" {
        let text_end_x = r.x as f64 + offset_x + text_width;
        let stroke_width_deco = (font_size * 0.1).max(1.0) as f64;
        if text_decoration.contains("underline") {
            let line_y = baseline_y + font_size as f64 * 0.1;
            scene.stroke(&vello::kurbo::Stroke::new(stroke_width_deco), Affine::IDENTITY, text_color, None, &vello::kurbo::Line::new((r.x as f64 + offset_x, line_y), (text_end_x, line_y)));
        }
    }
}

fn parse_color(name: &str) -> Color {
    let name = name.trim().to_lowercase();
    if name.starts_with('#') {
        let hex = &name[1..];
        if hex.len() == 6 {
            if let Ok(c) = u32::from_str_radix(hex, 16) {
                return Color::from_rgb8(((c >> 16) & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, (c & 0xFF) as u8);
            }
        }
    }
    match name.as_str() {
        "white" | "#fff" => Color::WHITE,
        "black" | "#000" => Color::BLACK,
        "red" => Color::from_rgb8(255, 0, 0),
        "blue" => Color::from_rgb8(0, 0, 255),
        "green" => Color::from_rgb8(0, 128, 0),
        "transparent" => Color::TRANSPARENT,
        "aurora-cyan" => Color::from_rgb8(0x88, 0xC0, 0xD0),
        "coal" => Color::from_rgb8(0x2E, 0x34, 0x40),
        "rust" => Color::from_rgb8(0xBF, 0x61, 0x6A),
        _ => Color::from_rgb8(0x4C, 0x56, 0x6A),
    }
}

fn paint_image(layout_box: &LayoutBox, scene: &mut Scene) {
    let r = layout_box.rect();
    let k_rect = KRect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(200, 200, 200), None, &k_rect);
}
