//! GPU glyph atlas for efficient text rendering
//!
//! Single packed texture containing all glyphs for a font.
//! Glyphs are pre-rasterized and packed using a simple guillotine algorithm.

use std::collections::HashMap;

/// Glyph metrics within the atlas
#[derive(Clone, Copy, Debug)]
pub struct GlyphMetrics {
    /// Top-left position in atlas texture (pixels)
    pub x: u32,
    pub y: u32,
    /// Glyph dimensions in atlas
    pub width: u32,
    pub height: u32,
    /// Offset from baseline when rendering (signed)
    pub x_offset: i32,
    pub y_offset: i32,
    /// Horizontal advance to next glyph
    pub advance_width: f32,
    /// UV coordinates in normalized [0, 1] space
    pub uv_min: (f32, f32),
    pub uv_max: (f32, f32),
}

/// Packed glyph atlas texture
pub struct GlyphAtlas {
    /// Atlas texture data (RGBA8)
    pub texture: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Glyph metrics keyed by character
    glyphs: HashMap<char, GlyphMetrics>,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        GlyphAtlas {
            texture: vec![0u8; (width * height * 4) as usize],
            width,
            height,
            glyphs: HashMap::new(),
        }
    }

    /// Register a glyph in the atlas
    pub fn register_glyph(
        &mut self,
        ch: char,
        bitmap: &[u8],
        glyph_width: u32,
        glyph_height: u32,
        x_offset: i32,
        y_offset: i32,
        advance_width: f32,
        atlas_x: u32,
        atlas_y: u32,
    ) {
        // Copy bitmap into atlas at (atlas_x, atlas_y)
        let atlas_stride = self.width * 4;
        let bitmap_stride = glyph_width;

        for row in 0..glyph_height {
            let src_offset = (row * bitmap_stride) as usize;
            let dst_offset = ((atlas_y + row) * self.width + atlas_x) as usize * 4;

            for col in 0..glyph_width {
                let src = src_offset + col as usize;
                let dst = dst_offset + col as usize * 4;

                if src < bitmap.len() && dst + 3 < self.texture.len() {
                    let alpha = bitmap[src];
                    // Write RGBA (white text with varying alpha)
                    self.texture[dst] = 255;     // R
                    self.texture[dst + 1] = 255; // G
                    self.texture[dst + 2] = 255; // B
                    self.texture[dst + 3] = alpha; // A
                }
            }
        }

        // Compute normalized UV coordinates
        let uv_min = (
            atlas_x as f32 / self.width as f32,
            atlas_y as f32 / self.height as f32,
        );
        let uv_max = (
            (atlas_x + glyph_width) as f32 / self.width as f32,
            (atlas_y + glyph_height) as f32 / self.height as f32,
        );

        let metrics = GlyphMetrics {
            x: atlas_x,
            y: atlas_y,
            width: glyph_width,
            height: glyph_height,
            x_offset,
            y_offset,
            advance_width,
            uv_min,
            uv_max,
        };

        self.glyphs.insert(ch, metrics);
    }

    /// Get metrics for a character, or None if not in atlas
    pub fn get_glyph(&self, ch: char) -> Option<GlyphMetrics> {
        self.glyphs.get(&ch).copied()
    }

    /// Get all registered glyphs
    pub fn glyphs(&self) -> &HashMap<char, GlyphMetrics> {
        &self.glyphs
    }
}

/// Simple guillotine packing for atlas layout
pub struct AtlasPacker {
    width: u32,
    height: u32,
    rows: Vec<PackRow>,
}

struct PackRow {
    y: u32,
    height: u32,
    x_cursor: u32,
}

impl AtlasPacker {
    pub fn new(width: u32, height: u32) -> Self {
        AtlasPacker {
            width,
            height,
            rows: vec![],
        }
    }

    /// Pack a glyph and return its (x, y) position in the atlas
    pub fn pack(&mut self, glyph_width: u32, glyph_height: u32) -> Option<(u32, u32)> {
        // Try to fit in existing row
        for row in &mut self.rows {
            if row.x_cursor + glyph_width <= self.width && glyph_height <= row.height {
                let x = row.x_cursor;
                let y = row.y;
                row.x_cursor += glyph_width;
                return Some((x, y));
            }
        }

        // Create new row if space available
        let next_y = self.rows.iter().map(|r| r.y + r.height).max().unwrap_or(0);
        if next_y + glyph_height <= self.height {
            let x = 0;
            let y = next_y;
            self.rows.push(PackRow {
                y,
                height: glyph_height,
                x_cursor: glyph_width,
            });
            return Some((x, y));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packer() {
        let mut packer = AtlasPacker::new(256, 256);

        // Pack several glyphs
        let pos1 = packer.pack(16, 16);
        assert!(pos1.is_some());

        let pos2 = packer.pack(16, 16);
        assert!(pos2.is_some());

        // They should be adjacent in the same row
        assert_eq!(pos1.unwrap().1, pos2.unwrap().1);
    }

    #[test]
    fn test_atlas_registration() {
        let mut atlas = GlyphAtlas::new(512, 512);

        // Create a simple glyph bitmap
        let bitmap = vec![255; 16 * 16];

        atlas.register_glyph(
            'A',
            &bitmap,
            16, 16,
            0, 0,
            10.0,
            0, 0,
        );

        let metrics = atlas.get_glyph('A');
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.width, 16);
        assert_eq!(m.height, 16);
        assert_eq!(m.advance_width, 10.0);
    }
}
