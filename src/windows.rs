use crate::{Frame, FrameInfo};

use std::time::Duration;

use wgc::*;
use windows::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{HMONITOR, MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint},
};

fn get_primary_monitor() -> HMONITOR {
    unsafe {
        let point = POINT::default();
        MonitorFromPoint(point, MONITOR_DEFAULTTOPRIMARY)
    }
}

fn create_wgc(frame_rate: u32) -> Result<Wgc, WgcError> {
    Wgc::new_from_monitor(get_primary_monitor())?
        .pixel_format(PixelFormat::RGBA8)
        .frame_queue_length(2)
        .capture_cursor(true)
        .min_update_interval(Duration::from_secs(1) / frame_rate)
        .build()
}

fn map_frame(frame: wgc::Frame) -> Result<Frame, WgcError> {
    let FrameSize { width, height } = frame.size()?;
    let bytes = frame.pixels()?;
    Ok(Frame {
        bytes,
        info: FrameInfo {
            width: width,
            height: height,
            stride: 4 * width,
            format: crate::Format::Rgba8,
        },
    })
}

/// Uses `WGC` to capture video.
pub fn capture_video(
    frame_rate: u32,
) -> Result<impl Iterator<Item = Result<Frame, WgcError>> + Send, WgcError> {
    let (init_sender, init_receiver) = std::sync::mpsc::sync_channel(0);
    let (sender, receiver) = std::sync::mpsc::sync_channel(0);
    std::thread::spawn(move || {
        let wgc = match create_wgc(frame_rate) {
            Ok(wgc) => wgc,
            Err(err) => {
                let _ = init_sender.send(Err(err));
                return;
            }
        };
        let Ok(()) = init_sender.send(Ok(())) else {
            return;
        };
        for frame_result in wgc {
            let item = match frame_result {
                Ok(frame) => map_frame(frame),
                Err(err) => Err(err),
            };
            let Ok(()) = sender.send(item) else {
                break;
            };
        }
    });
    init_receiver.recv().unwrap().map(|_| receiver.into_iter())
}
