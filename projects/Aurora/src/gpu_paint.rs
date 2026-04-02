use crate::layout::LayoutBox;
use vello::kurbo::RoundedRect;
use vello::peniko::{Color, Fill};
use vello::Scene;

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

    // Determine border radius
    let radius = if let Some(radius_str) = styles.get("border-radius") {
        radius_str.trim_end_matches("px").parse::<f32>().unwrap_or(6.0) as f64
    } else {
        6.0
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

    let font_size = styles.font_size_px().filter(|&s| s > 0.0).unwrap_or(16.0);
    let scale = font_size as f64 / 8.0;

    let bold = styles.font_weight() == "bold" || styles.font_weight() == "bolder" || styles.font_weight() == "700" || styles.font_weight().parse::<i32>().unwrap_or(400) >= 600;
    let italic = styles.font_style() == "italic";
    let text_align = styles.text_align();
    let text_decoration = styles.get("text-decoration").unwrap_or("none");

    // Calculate text width for alignment
    let text_width = text.chars().count() as f64 * 8.0 * scale;
    let box_width = r.width as f64;

    let offset_x = match text_align {
        crate::css::TextAlign::Center => (box_width - text_width).max(0.0) / 2.0,
        crate::css::TextAlign::Right => (box_width - text_width).max(0.0),
        crate::css::TextAlign::Left => 0.0,
    };

    // Vertical alignment: baseline is at 75% of the em height
    let baseline_y = r.y as f64 + scale * 5.5;

    // Use stroked outlines for smooth anti-aliased appearance
    let stroke_width = (scale * 0.3).max(0.8);

    for (i, ch) in text.chars().enumerate() {
        let char_x = r.x as f64 + offset_x + (i as f64 * 8.0 * scale);
        let glyph = crate::font::get_glyph(ch);

        for (row, &bits) in glyph.iter().enumerate() {
            let py = baseline_y - (6.0 - row as f64) * scale;
            let italic_offset = if italic { (7.0 - row as f64) / 3.0 * scale } else { 0.0 };

            for col in 0..8 {
                if (bits & (1 << (7 - col))) != 0 {
                    let px = char_x + (col as f64 * scale) + italic_offset;
                    let p_width = if bold { 1.2 * scale } else { 1.0 * scale };

                    // Draw as stroked rounded rectangle for smooth edges
                    let rect = vello::kurbo::Rect::new(px, py, px + p_width, py + scale);
                    let rounded_rect = RoundedRect::from_rect(rect, scale * 0.2);

                    scene.stroke(
                        &vello::kurbo::Stroke::new(stroke_width),
                        vello::kurbo::Affine::IDENTITY,
                        text_color,
                        None,
                        &rounded_rect,
                    );
                }
            }
        }
    }

    // Draw text decorations
    if text_decoration != "none" {
        let text_end_x = r.x as f64 + offset_x + text_width;
        let stroke_width = (scale * 0.5).max(1.0);

        if text_decoration.contains("underline") {
            let line_y = baseline_y + scale * 1.5;
            scene.stroke(
                &vello::kurbo::Stroke::new(stroke_width),
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
            let line_y = baseline_y + scale * 0.5;
            scene.stroke(
                &vello::kurbo::Stroke::new(stroke_width),
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

fn paint_text(layout_box: &LayoutBox, text: &str, scene: &mut Scene) {
    paint_text_with_opacity(layout_box, text, scene, 1.0);
}

fn paint_image(layout_box: &LayoutBox, scene: &mut Scene) {
    let r = layout_box.rect();
    let styles = layout_box.styles();
    let bg_color = parse_color(styles.get("background-color").unwrap_or("#e0e0e0"));
    let border_color = Color::from_rgb8(128, 128, 128);

    let k_rect = vello::kurbo::Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);

    scene.fill(
        Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        bg_color,
        None,
        &k_rect,
    );

    // Draw border
    scene.stroke(
        &vello::kurbo::Stroke::new(2.0),
        vello::kurbo::Affine::IDENTITY,
        border_color,
        None,
        &vello::kurbo::Rect::new(r.x as f64 + 1.0, r.y as f64 + 1.0, (r.x + r.width) as f64 - 1.0, (r.y + r.height) as f64 - 1.0),
    );

    // Draw a "picture frame" icon (diagonal lines forming an X)
    let stroke = vello::kurbo::Stroke::new(1.5);
    let pad = 4.0;
    scene.stroke(
        &stroke,
        vello::kurbo::Affine::IDENTITY,
        border_color,
        None,
        &vello::kurbo::Line::new(
            vello::kurbo::Point::new(r.x as f64 + pad, r.y as f64 + pad),
            vello::kurbo::Point::new((r.x + r.width) as f64 - pad, (r.y + r.height) as f64 - pad)
        )
    );
    scene.stroke(
        &stroke,
        vello::kurbo::Affine::IDENTITY,
        border_color,
        None,
        &vello::kurbo::Line::new(
            vello::kurbo::Point::new((r.x + r.width) as f64 - pad, r.y as f64 + pad),
            vello::kurbo::Point::new(r.x as f64 + pad, (r.y + r.height) as f64 - pad)
        )
    );
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
