use std::time::{Duration, Instant};

use log::debug;

mod wayland;
use wayland::WaylandCapture;

use crate::Yuv420Image;

fn capture_image(capture: &mut WaylandCapture) -> Yuv420Image {
    match capture.capture_output(0) {
        Ok(image) => image,
        Err(err) => panic!("Failed to capture screen: {err}"),
    }
}

// TODO: show_cursor: bool parameter
pub fn capture_video(frame_rate: u64) -> impl Iterator<Item = Yuv420Image> {
    let mut capture =
        WaylandCapture::connect().expect("Cannot connect to Wayland and X11 is not yet supported");
    let mut last = Instant::now();
    let frame_duration = Duration::from_nanos(1_000_000_000 / frame_rate);

    std::iter::from_fn(move || {
        let now_frame = Instant::now();
        debug!(
            "Time since last frame: {:.2}ms",
            (now_frame - last).as_micros() as f32 / 1000.0
        );
        let diff = now_frame.duration_since(last);
        if diff < frame_duration {
            std::thread::sleep(frame_duration - diff);
        }
        let now_capture = Instant::now();
        let frame = capture_image(&mut capture);
        debug!(
            "Frame capture time: {:.2}ms",
            (Instant::now() - now_capture).as_micros() as f32 / 1000.0
        );
        last = now_frame;
        Some(frame)
    })
}
