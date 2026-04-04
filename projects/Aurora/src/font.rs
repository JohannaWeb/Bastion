//! True font support with pre-baked glyph atlas and shaping
//!
//! This module provides TrueType font loading, shaping via rustybuzz,
//! and a unified glyph atlas for efficient GPU rendering.

use std::sync::OnceLock;
use crate::atlas::{GlyphAtlas, AtlasPacker};
use ab_glyph::{Font, FontRef, PxScale};
use rustybuzz::{Face, UnicodeBuffer, shape};

// Global glyph atlas initialized on first use
static GLYPH_ATLAS: OnceLock<GlyphAtlas> = OnceLock::new();
static FONT_DATA: &[u8] = include_bytes!("../fonts/default.ttf");
static FONT_FACE: OnceLock<Face<'static>> = OnceLock::new();

const ATLAS_BASE_SIZE: f32 = 32.0;

fn get_font_face() -> &'static Face<'static> {
    FONT_FACE.get_or_init(|| {
        Face::from_slice(FONT_DATA, 0).expect("Failed to parse font for shaping")
    })
}

fn get_ab_font() -> FontRef<'static> {
    FontRef::try_from_slice(FONT_DATA).expect("Failed to parse font for rasterization")
}

pub struct AtlasBuilder;

impl AtlasBuilder {
    /// Build the pre-baked glyph atlas for ASCII + extended (0-255)
    pub fn build() -> GlyphAtlas {
        let base_size = ATLAS_BASE_SIZE;
        let font = get_ab_font();
        let scale = PxScale::from(base_size);

        let atlas_width = 1024;
        let atlas_height = 1024;
        let mut atlas = GlyphAtlas::new(atlas_width, atlas_height);
        let mut packer = AtlasPacker::new(atlas_width, atlas_height);

        // Rasterize common characters (0-255)
        for code in 0u32..256 {
            if let Some(ch) = char::from_u32(code) {
                let glyph_id = font.glyph_id(ch);
                let glyph = glyph_id.with_scale(scale);
                
                if let Some(outline) = font.outline_glyph(glyph) {
                    let bounds = outline.px_bounds();
                    let width = bounds.width() as u32;
                    let height = bounds.height() as u32;
                    
                    if width > 0 && height > 0 {
                        let mut bitmap = vec![0u8; (width * height) as usize];
                        outline.draw(|x, y, v| {
                            let idx = (y * width + x) as usize;
                            if idx < bitmap.len() {
                                bitmap[idx] = (v * 255.0) as u8;
                            }
                        });

                        if let Some((atlas_x, atlas_y)) = packer.pack(width, height) {
                            // Scale factor from font units to pixels for atlas
                            let upem = font.units_per_em().unwrap_or(1000.0);
                            let advance = font.h_advance_unscaled(glyph_id) * (base_size / upem);
                            
                            atlas.register_glyph(
                                ch,
                                &bitmap,
                                width,
                                height,
                                bounds.min.x as i32,
                                bounds.min.y as i32,
                                advance,
                                atlas_x,
                                atlas_y,
                            );
                        }
                    }
                } else if ch == ' ' {
                    // Space has no outline but has advance
                    let upem = font.units_per_em().unwrap_or(1000.0);
                    let advance = font.h_advance_unscaled(glyph_id) * (base_size / upem);
                    atlas.register_glyph(
                        ch,
                        &[],
                        0, 0,
                        0, 0,
                        advance,
                        0, 0,
                    );
                }
            }
        }

        atlas
    }
}

fn get_glyph_atlas() -> &'static GlyphAtlas {
    GLYPH_ATLAS.get_or_init(AtlasBuilder::build)
}

pub fn get_glyph_metrics(ch: char) -> Option<crate::atlas::GlyphMetrics> {
    let atlas = get_glyph_atlas();
    atlas.get_glyph(ch)
}

pub fn measure_text(text: &str, font_size: f32) -> f32 {
    // Keep layout width deterministic even though rendering uses shaped glyphs.
    text.chars().count() as f32 * font_size
}

#[derive(Debug, Clone)]
pub struct RasterGlyph {
    pub width: u32,
    pub height: u32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub bitmap: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub ch: char,
    pub x: f32,
    pub y_offset: f32,
}

#[derive(Debug, Clone)]
pub struct TextRun {
    pub glyphs: Vec<PositionedGlyph>,
    pub width: f32,
}

pub fn layout_text_run(text: &str, font_size: f32) -> TextRun {
    let face = get_font_face();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    
    // Perform shaping
    let glyph_buffer = shape(face, &[], buffer);
    let infos = glyph_buffer.glyph_infos();
    let positions = glyph_buffer.glyph_positions();
    
    let mut glyphs = Vec::new();
    let mut cursor_x = 0.0;
    
    // Scale factor from font units to pixels
    let upem = face.units_per_em() as f32;
    let scale = font_size / upem;
    
    let text_chars: Vec<char> = text.chars().collect();
    
    for (i, (_info, pos)) in infos.iter().zip(positions.iter()).enumerate() {
        let ch = text_chars.get(i).copied().unwrap_or(' ');
        
        glyphs.push(PositionedGlyph {
            ch,
            x: cursor_x + (pos.x_offset as f32 * scale),
            y_offset: pos.y_offset as f32 * scale,
        });
        
        cursor_x += pos.x_advance as f32 * scale;
    }

    TextRun {
        glyphs,
        width: cursor_x,
    }
}

pub fn get_atlas_texture() -> (&'static [u8], u32, u32) {
    let atlas = get_glyph_atlas();
    (&atlas.texture, atlas.width, atlas.height)
}

pub fn rasterize_glyph(ch: char, font_size: f32) -> Option<RasterGlyph> {
    if ch == ' ' || ch == '\n' || ch == '\t' {
        return None;
    }

    let font = get_ab_font();
    let glyph_id = font.glyph_id(ch);
    let glyph = glyph_id.with_scale(PxScale::from(font_size));
    let outline = font.outline_glyph(glyph)?;
    let bounds = outline.px_bounds();
    let width = bounds.width().ceil().max(0.0) as u32;
    let height = bounds.height().ceil().max(0.0) as u32;
    if width == 0 || height == 0 {
        return None;
    }

    let mut bitmap = vec![0u8; (width * height) as usize];
    outline.draw(|x, y, coverage| {
        let idx = (y * width + x) as usize;
        if let Some(pixel) = bitmap.get_mut(idx) {
            *pixel = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
        }
    });

    Some(RasterGlyph {
        width,
        height,
        x_offset: bounds.min.x.floor() as i32,
        y_offset: bounds.min.y.floor() as i32,
        bitmap,
    })
}

// For compatibility with old system if needed
pub fn get_glyph(_ch: char) -> &'static [u8; 8] {
    &[0; 8]
}
