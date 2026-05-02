use dracon_terminal_engine::backend::tty::poll_input;
use dracon_terminal_engine::compositor::engine::Compositor;
use dracon_terminal_engine::compositor::plane::Plane;
use dracon_terminal_engine::core::terminal::Terminal;
use dracon_terminal_engine::input::event::{Event, KeyCode, KeyEvent};
use dracon_terminal_engine::input::parser::Parser;
use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut term = Terminal::new(io::stdout())?;

    // Enable SGR Mouse (1006) + Any Event (1003)
    write!(term, "\x1b[?1000h\x1b[?1003h\x1b[?1006h\x1b[?25l")?;
    term.flush()?;

    let (mut w, mut h) = dracon_terminal_engine::backend::tty::get_window_size(term.as_fd())?;
    let mut compositor = Compositor::new(w, h);
    let mut parser = Parser::new();
    let mut stdin = io::stdin();

    let mut x_pos = 10.0;
    let mut last_tick = Instant::now();

    // FPS Counter
    let mut frames = 0;
    let mut fps = 0;
    let mut fps_timer = Instant::now();
    let target_fps = 60.0;
    let _frame_duration = Duration::from_secs_f32(1.0 / target_fps);

    loop {
        // 1. Poll Input (Non-blocking)
        if poll_input(term.as_fd(), 0)? {
            // 0ms timeout = instant check
            let mut buf = [0u8; 128];
            if let Ok(n) = stdin.read(&mut buf) {
                for &byte in &buf[..n] {
                    if let Some(event) = parser.advance(byte) {
                        if let Event::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            ..
                        }) = event
                        {
                            write!(term, "\x1b[?25h")?;
                            return Ok(());
                        }
                    }
                }
            }
        }

        // 2. Resize Check
        if let Ok((new_w, new_h)) =
            dracon_terminal_engine::backend::tty::get_window_size(term.as_fd())
        {
            if new_w != w || new_h != h {
                w = new_w;
                h = new_h;
                compositor.resize(w, h);
            }
        }

        // 3. Update State (Time-based animation)
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        if dt >= 1.0 / target_fps {
            last_tick = now;

            x_pos += 20.0 * dt; // Move 20 chars per second
            if x_pos >= w as f32 {
                x_pos = 0.0;
            }

            // 4. Render
            compositor.planes.clear();
            let mut p = Plane::new(1, w, h);
            let msg = format!("FPS: {} | Res: {}x{} | X: {:.1}", fps, w, h, x_pos);
            p.put_str(0, 0, &msg);

            p.put_str(x_pos as u16, h / 2, "🚀 IO Writer");

            compositor.add_plane(p);
            compositor.render(term.inner())?;

            frames += 1;
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }

        if fps_timer.elapsed().as_secs() >= 1 {
            fps = frames;
            frames = 0;
            fps_timer = Instant::now();
        }
    }
}
