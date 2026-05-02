pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

pub struct ShapeGenerator;

impl ShapeGenerator {
    /// Generates a filled rounded rectangle.
    pub fn rounded_rect(width: u32, height: u32, color: Color, radius: u32) -> Vec<u8> {
        let mut buffer = Vec::with_capacity((width * height * 4) as usize);
        let r_sq = (radius as f32).powi(2);

        for y in 0..height {
            for x in 0..width {
                // Determine if pixel is inside the rounded corner
                let mut inside = true;

                // Top-Left
                if x < radius && y < radius {
                    let d = (x as f32 - radius as f32 + 0.5).powi(2)
                        + (y as f32 - radius as f32 + 0.5).powi(2);
                    if d > r_sq {
                        inside = false;
                    }
                }
                // Top-Right
                else if x >= width - radius && y < radius {
                    let d = (x as f32 - (width - radius) as f32 - 0.5).powi(2)
                        + (y as f32 - radius as f32 + 0.5).powi(2);
                    if d > r_sq {
                        inside = false;
                    }
                }
                // Bottom-Left
                else if x < radius && y >= height - radius {
                    let d = (x as f32 - radius as f32 + 0.5).powi(2)
                        + (y as f32 - (height - radius) as f32 - 0.5).powi(2);
                    if d > r_sq {
                        inside = false;
                    }
                }
                // Bottom-Right
                else if x >= width - radius && y >= height - radius {
                    let d = (x as f32 - (width - radius) as f32 - 0.5).powi(2)
                        + (y as f32 - (height - radius) as f32 - 0.5).powi(2);
                    if d > r_sq {
                        inside = false;
                    }
                }

                if inside {
                    buffer.push(color.r);
                    buffer.push(color.g);
                    buffer.push(color.b);
                    buffer.push(color.a);
                } else {
                    // Transparent
                    buffer.push(0);
                    buffer.push(0);
                    buffer.push(0);
                    buffer.push(0);
                }
            }
        }
        buffer
    }

    /// Generates a vertical gradient rectangle.
    pub fn gradient_vertical(width: u32, height: u32, start: Color, end: Color) -> Vec<u8> {
        let mut buffer = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            let t = y as f32 / height.max(1) as f32;
            let r = (start.r as f32 * (1.0 - t) + end.r as f32 * t) as u8;
            let g = (start.g as f32 * (1.0 - t) + end.g as f32 * t) as u8;
            let b = (start.b as f32 * (1.0 - t) + end.b as f32 * t) as u8;
            let a = (start.a as f32 * (1.0 - t) + end.a as f32 * t) as u8;

            for _x in 0..width {
                buffer.push(r);
                buffer.push(g);
                buffer.push(b);
                buffer.push(a);
            }
        }
        buffer
    }

    /// Generates a horizontal gradient rectangle.
    pub fn gradient_horizontal(width: u32, height: u32, start: Color, end: Color) -> Vec<u8> {
        let mut buffer = Vec::with_capacity((width * height * 4) as usize);

        for _y in 0..height {
            for x in 0..width {
                let t = x as f32 / width.max(1) as f32;
                let r = (start.r as f32 * (1.0 - t) + end.r as f32 * t) as u8;
                let g = (start.g as f32 * (1.0 - t) + end.g as f32 * t) as u8;
                let b = (start.b as f32 * (1.0 - t) + end.b as f32 * t) as u8;
                let a = (start.a as f32 * (1.0 - t) + end.a as f32 * t) as u8;

                buffer.push(r);
                buffer.push(g);
                buffer.push(b);
                buffer.push(a);
            }
        }
        buffer
    }
}
