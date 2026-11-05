use log::info;
use xcap::Monitor;

use crate::Error;

fn get_primary_monitor() -> Result<Monitor, Error> {
    let monitors = Monitor::all()?;
    let mut error = None;
    for m in monitors {
        match m.is_primary() {
            Ok(true) => return Ok(m),
            Ok(false) => continue,
            Err(err) => error = Some(err),
        }
    }
    Err(error.map(Error::from).unwrap_or(Error::NoMonitorDetected))
}

#[cfg(target_os = "windows")]
mod windows {
    use windows::Win32::{
        Foundation::POINT,
        UI::WindowsAndMessaging::{CURSOR_SHOWING, CURSORINFO, GetCursorInfo, GetCursorPos},
    };
    use windows::core::Result;

    pub fn draw_cursor(bytes: &mut [u8], width: u32, height: u32) -> Result<()> {
        unsafe {
            let mut point = POINT::default();
            GetCursorPos(&mut point)?;
            let POINT { x, y } = point;

            let mut cursor_info = CURSORINFO {
                cbSize: size_of::<CURSORINFO>() as _,
                ..Default::default()
            };
            GetCursorInfo(&mut cursor_info)?;

            if cursor_info.flags != CURSOR_SHOWING {
                return Ok(());
            }
            // TODO: draw the actual cursor icon

            let color = [200, 200, 200, 255];
            let cursor_size = 10;

            if x < 0 || x < width as i32 || y < 0 {
                return Ok(());
            }
            let x = x as u32;
            let y = y as u32;
            for y in y..y + cursor_size {
                if y >= height {
                    break;
                }
                let start = ((y * width + x.min(width)) * 4) as usize;
                let end = ((y * width + (x + cursor_size).min(width)) * 4) as usize;
                bytes[start..end].chunks_exact_mut(4).for_each(|chunk| {
                    chunk.copy_from_slice(&color);
                });
            }
            Ok(())
        }
    }
}

pub fn capture_video(frame_rate: u64) -> Result<impl Iterator<Item = crate::Frame>, Error> {
    info!("Getting primary monitor");
    let monitor = get_primary_monitor()?;
    info!("Creating video recorder");
    let (recorder, receiver) = monitor.video_recorder()?;

    info!("Starting recording");
    recorder.start()?;
    let video = receiver.into_iter().map(
        |xcap::Frame {
             raw: bytes,
             width,
             height,
         }| {
            #[cfg(target_os = "windows")]
            let mut bytes = bytes;
            #[cfg(target_os = "windows")]
            if let Err(err) = windows::draw_cursor(&mut bytes, width, height) {
                log::warn!("Failed to draw cursor: {err}");
            };

            crate::Frame {
                bytes,
                info: crate::FrameInfo {
                    width,
                    height,
                    stride: width * 4,
                    format: crate::Format::Rgba8,
                },
            }
        },
    );
    info!("Video stream created");
    Ok(crate::subsample_video(video, frame_rate))
}
