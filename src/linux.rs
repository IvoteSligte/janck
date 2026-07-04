use std::time::{Duration, Instant};

use log::debug;

mod wayland;
use wayland::WaylandCapture;

use crate::{Rgb8Image, Yuv420Image};

pub struct State {
    capture: WaylandCapture,
}

impl State {
    pub fn new() -> Self {
        Self {
            capture: WaylandCapture::connect()
                .expect("Cannot connect to Wayland. X11 is not yet supported."),
        }
    }

    pub fn capture(&mut self) -> Rgb8Image {
        let shot = self
            .capture
            .capture_output(0)
            .expect("failed to capture screen");
        let width = shot.width();
        let height = shot.height();
        let mut bytes = shot.into_data();

        // convert RGBA to RGB
        for i in 0..bytes.len() / 4 {
            bytes[i * 3] = bytes[i * 4];
            bytes[i * 3 + 1] = bytes[i * 4 + 1];
            bytes[i * 3 + 2] = bytes[i * 4 + 2];
        }
        unsafe {
            bytes.set_len(bytes.len() / 4 * 3);
        }
        Rgb8Image {
            width,
            height,
            data: bytes,
        }
    }
}

pub fn capture_video(frame_rate: u64) -> impl Iterator<Item = Yuv420Image> {
    let mut state = State::new();
    let mut last = Instant::now();
    let frame_duration = Duration::from_nanos(1_000_000_000 / frame_rate);

    std::iter::from_fn(move || {
        let now = Instant::now();
        let diff = now.duration_since(last);
        debug!("Frame time: {}ms", diff.as_millis());
        if diff < frame_duration {
            std::thread::sleep(frame_duration - diff);
        }
        let frame = state.capture().to_yuv().unwrap();
        last = now;
        Some(frame)
    })
}
