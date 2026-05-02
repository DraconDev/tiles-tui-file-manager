use super::osc::simple_base64_encode;
use std::io::{self, Write};

#[derive(Clone, Debug, Default)]
pub struct ImageOptions {
    pub x: Option<u32>,
    pub y: Option<u32>,
    pub z_index: i32,
    pub placement_id: Option<u32>,
    pub columns: Option<u32>,
    pub rows: Option<u32>,
}

/// Kitty Graphics Protocol Implementation
pub struct ImageProtocol;

impl ImageProtocol {
    /// Transmit RGBA image data to the terminal but do not display it yet.
    /// This uses action = 't'.
    pub fn transmit_rgba<W: Write>(
        writer: &mut W,
        data: &[u8],
        width: u32,
        height: u32,
        id: u32,
    ) -> io::Result<()> {
        let encoded = simple_base64_encode(data);
        let chunks = encoded.as_bytes().chunks(4096);
        let total_chunks = chunks.len();

        for (i, chunk) in chunks.enumerate() {
            write!(writer, "\x1b_G")?;

            if i == 0 {
                write!(writer, "a=t,f=32,s={},v={},i={},q=2", width, height, id)?;
            }

            let more = if i < total_chunks - 1 { 1 } else { 0 };
            write!(writer, ",m={};", more)?;
            writer.write_all(chunk)?;
            write!(writer, "\x1b\\")?;
        }
        Ok(())
    }

    /// Place a previously transmitted image at the current cursor position.
    /// This uses action = 'p'.
    pub fn put_image<W: Write>(writer: &mut W, id: u32, options: ImageOptions) -> io::Result<()> {
        write!(writer, "\x1b_Ga=p,i={}", id)?;

        // Z-index
        write!(writer, ",z={}", options.z_index)?;
        // Placement ID
        if let Some(pid) = options.placement_id {
            write!(writer, ",p={}", pid)?;
        }
        // Columns (scaling)
        if let Some(c) = options.columns {
            write!(writer, ",c={}", c)?;
        }
        // Rows (scaling)
        if let Some(r) = options.rows {
            write!(writer, ",r={}", r)?;
        }
        // X offset (cells)
        if let Some(x) = options.x {
            write!(writer, ",x={}", x)?;
        }
        // Y offset (cells)
        if let Some(y) = options.y {
            write!(writer, ",y={}", y)?;
        }

        write!(writer, "\x1b\\")
    }

    /// Transmit and Display an RGBA image at the current cursor position.
    /// This is the simplest display method (action = 'T').
    pub fn display_rgba<W: Write>(
        writer: &mut W,
        data: &[u8],
        width: u32,
        height: u32,
        id: u32,
        options: ImageOptions,
    ) -> io::Result<()> {
        let encoded = simple_base64_encode(data);
        let chunks = encoded.as_bytes().chunks(4096);
        let total_chunks = chunks.len();

        for (i, chunk) in chunks.enumerate() {
            write!(writer, "\x1b_G")?;

            // Control keys only on first chunk
            if i == 0 {
                write!(writer, "a=T,f=32,s={},v={},i={},q=2", width, height, id)?;
                // Z-index
                write!(writer, ",z={}", options.z_index)?;
                // Placement ID (if needed for modification later)
                if let Some(pid) = options.placement_id {
                    write!(writer, ",p={}", pid)?;
                }
                // X offset
                if let Some(x) = options.x {
                    write!(writer, ",x={}", x)?;
                }
                // Y offset
                if let Some(y) = options.y {
                    write!(writer, ",y={}", y)?;
                }
                // Columns (scaling)
                if let Some(c) = options.columns {
                    write!(writer, ",c={}", c)?;
                }
                // Rows (scaling)
                if let Some(r) = options.rows {
                    write!(writer, ",r={}", r)?;
                }
            }

            // m=1 if more chunks, m=0 if last
            let more = if i < total_chunks - 1 { 1 } else { 0 };
            write!(writer, ",m={};", more)?;

            // Payload
            writer.write_all(chunk)?;

            // Footer
            write!(writer, "\x1b\\")?;
        }

        Ok(())
    }

    /// Delete an image by ID
    pub fn delete_id<W: Write>(writer: &mut W, id: u32) -> io::Result<()> {
        write!(writer, "\x1b_Ga=d,d=i,i={}\x1b\\", id)
    }

    /// Delete all images
    pub fn delete_all<W: Write>(writer: &mut W) -> io::Result<()> {
        write!(writer, "\x1b_Ga=d,d=a\x1b\\")
    }
}
