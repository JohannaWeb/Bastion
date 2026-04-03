use crate::layout::LayoutBox;
use vello::kurbo::RoundedRect;
use vello::peniko::{Color, Fill};
use vello::Scene;
use std::sync::OnceLock;
use std::sync::Mutex;
use std::collections::HashMap;

// Cache for dominant image colors
static IMAGE_COLOR_CACHE: OnceLock<Mutex<HashMap<String, [u8; 3]>>> = OnceLock::new();

fn get_image_color_cache() -> &'static Mutex<HashMap<String, [u8; 3]>> {
    IMAGE_COLOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct GpuPainter;

impl GpuPainter {
    pub fn paint(layout_box: &LayoutBox, scene: &mut Scene) {
        Self::paint_with_opacity(layout_box, scene, 1.0);
    }

    fn paint_with_opacity(layout_box: &LayoutBox, scene: &mut Scene, parent_opacity: f32) {
        let styles = layout_box.styles();
        let opacity = styles.opacity();
        let effective_opacity = parent_opacity * opacity;

        // Skip painting if opacity is very low
        if effective_opacity < 0.01 {
            return;
        }

        // Skip painting if visibility is hidden
        if styles.visibility() == "hidden" {
            return;
        }

        let rect = layout_box.rect();

        if layout_box.is_viewport() {
            let bg_color = parse_color(styles.background_color().unwrap_or("white"));
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                bg_color,
                None,
                &vello::kurbo::Rect::new(rect.x as f64, rect.y as f64, (rect.x + rect.width) as f64, (rect.y + rect.height) as f64),
            );
        } else if layout_box.is_image() {
            paint_image(layout_box, scene);
        } else if let Some(text) = layout_box.text() {
            paint_text_with_opacity(layout_box, text, scene, effective_opacity);
        } else {
            paint_element_with_opacity(layout_box, scene, effective_opacity);
        }

        for child in layout_box.children() {
            Self::paint_with_opacity(child, scene, effective_opacity);
        }
    }
}

fn paint_element_with_opacity(layout_box: &LayoutBox, scene: &mut Scene, opacity: f32) {
    let r = layout_box.rect();
    let styles = layout_box.styles();

    let bg_color_name = styles.get("background-color").or_else(|| styles.get("background")).unwrap_or("transparent");
    let mut bg_color = parse_color(bg_color_name);

    let mut border_color = parse_color(styles.get("border-color").unwrap_or("black"));
    let border = layout_box.styles().border_width();

    // Apply opacity to colors
    bg_color.components[3] *= opacity;
    border_color.components[3] *= opacity;

    // Determine border radius (default 0 — only round if explicitly styled)
    let radius = if let Some(radius_str) = styles.get("border-radius") {
        radius_str.trim_end_matches("px").parse::<f32>().unwrap_or(0.0) as f64
    } else {
        0.0
    };

    let k_rect = vello::kurbo::Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
    let rounded_rect = RoundedRect::from_rect(k_rect, radius);

    // Draw box shadow if specified
    if let Some(shadow_str) = styles.get("box-shadow") {
        if shadow_str != "none" {
            let shadow_color = Color::from_rgba8(0, 0, 0, ((60.0 * opacity) as u8).min(255));
            let shadow_rect = vello::kurbo::Rect::new(
                (r.x + 3.0) as f64,
                (r.y + 3.0) as f64,
                (r.x + r.width + 3.0) as f64,
                (r.y + r.height + 3.0) as f64,
            );
            scene.fill(
                Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                shadow_color,
                None,
                &RoundedRect::from_rect(shadow_rect, radius.max(2.0)),
            );
        }
    }

    // Fill background
    if bg_color.components[3] > 0.0 {
        scene.fill(
            Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            bg_color,
            None,
            &rounded_rect,
        );
    }

    // Draw border
    if border.top > 0.0 {
        let stroke_width = border.top as f64;
        scene.stroke(
            &vello::kurbo::Stroke::new(stroke_width),
            vello::kurbo::Affine::IDENTITY,
            border_color,
            None,
            &rounded_rect,
        );
    }
}

fn paint_element(layout_box: &LayoutBox, scene: &mut Scene) {
    paint_element_with_opacity(layout_box, scene, 1.0);
}

fn paint_text_with_opacity(layout_box: &LayoutBox, text: &str, scene: &mut Scene, opacity: f32) {
    let r = layout_box.rect();
    let styles = layout_box.styles();
    let mut text_color = parse_color(styles.get("color").unwrap_or("black"));

    // Apply opacity to text color
    text_color.components[3] *= opacity;

    let mut font_size = styles.font_size_px().filter(|&s| s > 0.0).unwrap_or(16.0);

    // Apply global zoom if set
    if let Ok(zoom_str) = std::env::var("AURORA_ZOOM") {
        if let Ok(zoom) = zoom_str.parse::<f32>() {
            font_size *= zoom;
        }
    }

    let italic = styles.font_style() == "italic";
    let text_align = styles.text_align();
    let text_decoration = styles.get("text-decoration").unwrap_or("none");

    // Atlas base size is 32px, scale glyphs relative to it
    let atlas_size = 32.0;
    let scale_factor = (font_size / atlas_size) as f64;

    // Calculate text width using atlas metrics
    let mut text_width = 0.0;
    for ch in text.chars() {
        if let Some(metrics) = crate::font::get_glyph_metrics(ch) {
            text_width += (metrics.advance_width as f64 * scale_factor);
        }
    }

    let box_width = r.width as f64;

    let offset_x = match text_align {
        crate::css::TextAlign::Center => (box_width - text_width).max(0.0) / 2.0,
        crate::css::TextAlign::Right => (box_width - text_width).max(0.0),
        crate::css::TextAlign::Left => 0.0,
    };

    // Vertical alignment: baseline at 75% of em height
    let baseline_y = r.y as f64 + font_size as f64 * 0.75;
    let mut current_x = r.x as f64 + offset_x;

    for ch in text.chars() {
        if let Some(metrics) = crate::font::get_glyph_metrics(ch) {
            let glyph_width = (metrics.width as f64) * scale_factor;
            let glyph_height = (metrics.height as f64) * scale_factor;

            if glyph_width > 0.0 && glyph_height > 0.0 {
                let glyph_x = current_x + (metrics.x_offset as f64 * scale_factor);
                let glyph_y = baseline_y - (metrics.height as f64 * 0.75 * scale_factor);

                // Draw pixel-by-pixel for crisp appearance
                render_glyph_from_atlas(
                    scene,
                    ch,
                    glyph_x,
                    glyph_y,
                    scale_factor,
                    text_color,
                    italic,
                );
            }

            current_x += (metrics.advance_width as f64 * scale_factor);
        }
    }

    // Draw text decorations
    if text_decoration != "none" {
        let text_end_x = r.x as f64 + offset_x + text_width;
        let stroke_width_deco = (font_size * 0.15).max(0.8);

        if text_decoration.contains("underline") {
            let line_y = baseline_y + font_size as f64 * 0.15;
            scene.stroke(
                &vello::kurbo::Stroke::new(stroke_width_deco as f64),
                vello::kurbo::Affine::IDENTITY,
                text_color,
                None,
                &vello::kurbo::Line::new(
                    vello::kurbo::Point::new(r.x as f64 + offset_x, line_y),
                    vello::kurbo::Point::new(text_end_x, line_y),
                ),
            );
        }

        if text_decoration.contains("line-through") {
            let line_y = baseline_y - font_size as f64 * 0.2;
            scene.stroke(
                &vello::kurbo::Stroke::new(stroke_width_deco as f64),
                vello::kurbo::Affine::IDENTITY,
                text_color,
                None,
                &vello::kurbo::Line::new(
                    vello::kurbo::Point::new(r.x as f64 + offset_x, line_y),
                    vello::kurbo::Point::new(text_end_x, line_y),
                ),
            );
        }
    }
}

/// Render a glyph by sampling its bitmap from the atlas
fn render_glyph_from_atlas(
    scene: &mut Scene,
    ch: char,
    glyph_x: f64,
    glyph_y: f64,
    scale: f64,
    color: Color,
    italic: bool,
) {
    // Get the glyph bitmap from the font system
    let bitmap = crate::font::get_glyph(ch);

    // Render 8x8 bitmap
    for (row, &bits) in bitmap.iter().enumerate() {
        for col in 0..8 {
            if (bits & (1 << (7 - col))) != 0 {
                let px = glyph_x + (col as f64 * scale);
                let py = glyph_y + (row as f64 * scale);

                // Draw pixel as tiny rectangle
                let rect = vello::kurbo::Rect::new(
                    px,
                    py,
                    (px + scale).min(px + scale * 1.2),
                    (py + scale).min(py + scale * 1.2),
                );

                if italic {
                    let skew = (row as f64 / 8.0) * 0.2;
                    let affine = vello::kurbo::Affine::new([1.0, 0.0, skew * 0.1, 1.0, 0.0, 0.0]);
                    scene.fill(Fill::NonZero, affine, color, None, &rect);
                } else {
                    scene.fill(Fill::NonZero, vello::kurbo::Affine::IDENTITY, color, None, &rect);
                }
            }
        }
    }
}

fn paint_text(layout_box: &LayoutBox, text: &str, scene: &mut Scene) {
    paint_text_with_opacity(layout_box, text, scene, 1.0);
}

fn paint_image(layout_box: &LayoutBox, scene: &mut Scene) {
    let r = layout_box.rect();
    let k_rect = vello::kurbo::Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);

    // Try to fetch and analyze the image, fall back to placeholder if it fails
    let (bg_color, border_color) = if let Some(src) = layout_box.image_src() {
        let cache = get_image_color_cache();
        let mut cache_lock = cache.lock().unwrap();

        // Check if color is already cached
        if let Some(&color) = cache_lock.get(src) {
            (Color::from_rgb8(color[0], color[1], color[2]), Color::from_rgb8(100, 100, 100))
        } else {
            // Try to fetch and analyze the image
            let dominant_color = fetch_and_analyze_image(src);
            let (bg_col, border_col) = if let Some(color) = dominant_color {
                cache_lock.insert(src.to_string(), color);
                (Color::from_rgb8(color[0], color[1], color[2]), Color::from_rgb8(
                    (color[0] as i32 - 50).max(0) as u8,
                    (color[1] as i32 - 50).max(0) as u8,
                    (color[2] as i32 - 50).max(0) as u8,
                ))
            } else {
                // Fallback to light blue if image fetch/decode fails
                (Color::from_rgb8(220, 235, 250), Color::from_rgb8(100, 150, 200))
            };
            (bg_col, border_col)
        }
    } else {
        // No image source, use placeholder colors
        (Color::from_rgb8(220, 235, 250), Color::from_rgb8(100, 150, 200))
    };

    scene.fill(
        Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        bg_color,
        None,
        &k_rect,
    );

    // Draw rounded border
    let rounded_rect = RoundedRect::from_rect(k_rect, 4.0);
    scene.stroke(
        &vello::kurbo::Stroke::new(2.0),
        vello::kurbo::Affine::IDENTITY,
        border_color,
        None,
        &rounded_rect,
    );
}

fn fetch_and_analyze_image(url: &str) -> Option<[u8; 3]> {
    // Skip data URIs for now (would need to handle base64 decoding)
    if url.starts_with("data:") {
        return None;
    }

    // Create a dummy identity for network access
    let identity = opus::domain::Identity::new(
        "did:human:aurora",
        "Aurora",
        opus::domain::IdentityKind::Human,
        [opus::domain::Capability::NetworkAccess],
    );

    // For relative URLs, we can't resolve them without context, so skip for now
    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("/") {
        // Root-relative URL - we'd need the base URL to resolve, skip for now
        return None;
    } else {
        return None;
    };

    // Fetch the image bytes
    let bytes = match crate::fetch::fetch_bytes(&full_url, &identity) {
        Ok(b) => b,
        Err(_) => return None,
    };

    // Decode the image
    let img = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(_) => return None,
    };

    // Convert to RGBA for consistent processing
    let rgba_img = img.to_rgba8();

    // Calculate dominant color by averaging
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count = 0u64;

    for pixel in rgba_img.pixels() {
        // Skip very transparent pixels
        if pixel[3] < 128 {
            continue;
        }
        r_sum += pixel[0] as u64;
        g_sum += pixel[1] as u64;
        b_sum += pixel[2] as u64;
        count += 1;
    }

    if count == 0 {
        return None;
    }

    Some([
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
    ])
}

fn parse_color(name: &str) -> Color {
    let name = name.trim().to_lowercase();

    // Handle rgb() and rgba()
    if name.starts_with("rgba(") || name.starts_with("rgb(") {
        let is_rgba = name.starts_with("rgba");
        let start = if is_rgba { 5 } else { 4 };
        if let Some(end) = name.find(')') {
            let params_str = &name[start..end];
            let parts: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].split_whitespace().next().unwrap_or("0").parse::<u32>(),
                    parts[1].split_whitespace().next().unwrap_or("0").parse::<u32>(),
                    parts[2].split_whitespace().next().unwrap_or("0").parse::<u32>(),
                ) {
                    let alpha = if is_rgba && parts.len() > 3 {
                        parts[3].split_whitespace().next().unwrap_or("1").parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    return Color::from_rgba8((r & 0xFF) as u8, (g & 0xFF) as u8, (b & 0xFF) as u8, (alpha * 255.0) as u8);
                }
            }
        }
    }

    // Handle hsl() and hsla()
    if name.starts_with("hsla(") || name.starts_with("hsl(") {
        let is_hsla = name.starts_with("hsla");
        let start = if is_hsla { 5 } else { 4 };
        if let Some(end) = name.find(')') {
            let params_str = &name[start..end];
            let parts: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                if let (Ok(h), Ok(s), Ok(l)) = (
                    parts[0].split_whitespace().next().unwrap_or("0").parse::<f32>(),
                    parts[1].split_whitespace().next().unwrap_or("0").parse::<f32>(),
                    parts[2].split_whitespace().next().unwrap_or("0").parse::<f32>(),
                ) {
                    let alpha = if is_hsla && parts.len() > 3 {
                        parts[3].split_whitespace().next().unwrap_or("1").parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    return hsl_to_color(h % 360.0, s.clamp(0.0, 100.0), l.clamp(0.0, 100.0), alpha);
                }
            }
        }
    }

    if name.starts_with('#') {
        let hex = &name[1..];
        if hex.len() == 6 {
            if let Ok(c) = u32::from_str_radix(hex, 16) {
                return Color::from_rgb8(
                    ((c >> 16) & 0xFF) as u8,
                    ((c >> 8) & 0xFF) as u8,
                    (c & 0xFF) as u8,
                );
            }
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0);
            return Color::from_rgb8(r * 17, g * 17, b * 17);
        } else if hex.len() == 8 {
            if let Ok(c) = u32::from_str_radix(hex, 16) {
                return Color::from_rgba8(
                    ((c >> 24) & 0xFF) as u8,
                    ((c >> 16) & 0xFF) as u8,
                    ((c >> 8) & 0xFF) as u8,
                    (c & 0xFF) as u8,
                );
            }
        }
    }

    match name.as_str() {
        "white" | "#fff" | "#ffffff" => Color::WHITE,
        "black" | "#000" | "#000000" => Color::BLACK,
        "transparent" => Color::TRANSPARENT,
        "red" => Color::from_rgb8(255, 0, 0),
        "green" => Color::from_rgb8(0, 128, 0),
        "blue" => Color::from_rgb8(0, 0, 255),
        "gray" | "grey" => Color::from_rgb8(128, 128, 128),
        "lightgray" | "lightgrey" => Color::from_rgb8(211, 211, 211),
        "darkgray" | "darkgrey" => Color::from_rgb8(169, 169, 169),
        "aurora-cyan" => Color::from_rgb8(0x88, 0xC0, 0xD0),
        "coal" => Color::from_rgb8(0x2E, 0x34, 0x40),
        "ink" => Color::from_rgb8(0x3B, 0x42, 0x52),
        "paper" | "snow" => Color::from_rgb8(0xEC, 0xEF, 0xF4),
        "mist" => Color::from_rgb8(0xD8, 0xDE, 0xE9),
        "accent" => Color::from_rgb8(0x88, 0xC0, 0xD0),
        "rust" => Color::from_rgb8(0xBF, 0x61, 0x6A),
        "fern" => Color::from_rgb8(0xA3, 0xBE, 0x8C),
        "sky" => Color::from_rgb8(0x81, 0xA1, 0xC1),
        "gold" => Color::from_rgb8(0xEB, 0xCB, 0x8B),
        _ => Color::from_rgb8(0x4C, 0x56, 0x6A),
    }
}

fn hsl_to_color(h: f32, s: f32, l: f32, a: f32) -> Color {
    let s = s / 100.0;
    let l = l / 100.0;

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());

    let (r, g, b) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let m = l - c / 2.0;
    Color::from_rgba8(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
        (a * 255.0) as u8,
    )
}
