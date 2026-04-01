use crate::layout::LayoutBox;
use vello::kurbo::RoundedRect;
use vello::peniko::{Color, Fill};
use vello::Scene;

pub struct GpuPainter;

impl GpuPainter {
    pub fn paint(layout_box: &LayoutBox, scene: &mut Scene) {
        let styles = layout_box.styles();
        
        // Skip painting if opacity is very low
        if styles.opacity() < 0.1 {
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
            paint_text(layout_box, text, scene);
        } else {
            paint_element(layout_box, scene);
        }

        for child in layout_box.children() {
            Self::paint(child, scene);
        }
    }
}

fn paint_element(layout_box: &LayoutBox, scene: &mut Scene) {
    let r = layout_box.rect();
    let styles = layout_box.styles();
    
    let bg_color_name = styles.get("background-color").or_else(|| styles.get("background")).unwrap_or("transparent");
    let bg_color = parse_color(bg_color_name);
    
    let border_color_name = styles.get("border-color").unwrap_or("black");
    let border_color = parse_color(border_color_name);
    let border = layout_box.styles().border_width();
    
    let radius = 6.0; // Default rounded corners as in the old window.rs

    let k_rect = vello::kurbo::Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
    let rounded_rect = RoundedRect::from_rect(k_rect, radius);

    // Draw shadow for blocks
    if let Some(tag) = layout_box.tag_name() {
        if tag == "div" || tag == "section" || tag == "article" {
            // Simple shadow effect: a slightly larger, blurred rect behind
            let shadow_color = Color::from_rgba8(0, 0, 0, 40);
            let shadow_rect = vello::kurbo::Rect::new(
                (r.x + 2.0) as f64, 
                (r.y + 2.0) as f64, 
                (r.x + r.width + 2.0) as f64, 
                (r.y + r.height + 2.0) as f64
            );
            scene.fill(
                Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                shadow_color,
                None,
                &RoundedRect::from_rect(shadow_rect, radius),
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

fn paint_text(layout_box: &LayoutBox, text: &str, scene: &mut Scene) {
    let r = layout_box.rect();
    let styles = layout_box.styles();
    let text_color = parse_color(styles.get("color").unwrap_or("black"));
    
    let char_width = 8.0;
    let char_height = 12.0;
    
    for (i, _ch) in text.chars().enumerate() {
        let x = r.x as f64 + (i as f64 * char_width);
        let y = r.y as f64 + 2.0;
        
        let glyph_rect = vello::kurbo::Rect::new(x, y, x + char_width - 1.0, y + char_height);
        scene.fill(
            Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            text_color,
            None,
            &glyph_rect,
        );
    }
}

fn paint_image(layout_box: &LayoutBox, scene: &mut Scene) {
    let r = layout_box.rect();
    let bg_color = Color::from_rgb8(200, 200, 200);
    let k_rect = vello::kurbo::Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
    
    scene.fill(
        Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        bg_color,
        None,
        &k_rect,
    );
    
    // Draw a "cross" to indicate image
    let stroke = vello::kurbo::Stroke::new(1.0);
    scene.stroke(
        &stroke,
        vello::kurbo::Affine::IDENTITY,
        Color::from_rgb8(100, 100, 100),
        None,
        &vello::kurbo::Line::new(
            vello::kurbo::Point::new(r.x as f64, r.y as f64),
            vello::kurbo::Point::new((r.x + r.width) as f64, (r.y + r.height) as f64)
        )
    );
}

fn parse_color(name: &str) -> Color {
    let name = name.trim().to_lowercase();
    
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
        }
    }

    match name.as_str() {
        "white" | "#fff" | "#ffffff" => Color::WHITE,
        "black" | "#000" | "#000000" => Color::BLACK,
        "transparent" => Color::TRANSPARENT,
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
