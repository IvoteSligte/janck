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
                .expect("Cannot connect to Wayland. X11 is not yet supported"),
        }
    }

    pub fn capture(&mut self) -> Rgb8Image {
        match self.capture.capture_output(0) {
            Ok(image) => image,
            Err(err) => panic!("Failed to capture screen: {err}"),
        }
    }
}

pub fn capture_video(frame_rate: u64) -> impl Iterator<Item = Yuv420Image> {
    let mut state = State::new();
    let mut last = Instant::now();
    let frame_duration = Duration::from_nanos(1_000_000_000 / frame_rate);

    std::iter::from_fn(move || {
        let now_frame = Instant::now();
        let diff = now_frame.duration_since(last);
        if diff < frame_duration {
            std::thread::sleep(frame_duration - diff);
        }
        let now_capture = Instant::now();
        let frame = state.capture().to_yuv().unwrap();
        debug!(
            "Frame capture time: {}ms",
            (Instant::now() - now_capture).as_millis()
        );
        last = now_frame;
        Some(frame)
    })
}
