// NOTE: the linux wayland implementation is based on https://github.com/sunsided/miniscreenshot/miniscreenshot-wayland
// but there are also other implementations there that could be useful (miniscreenshot-x11, miniscreenshot-portal)

use std::time::{Duration, Instant};

use ::xcap::XCapError;
use log::{debug, trace};
use thiserror::Error;

#[cfg(target_os = "linux")]
pub mod wayland;
pub mod xcap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bgra8,
    Rgba8,
    // TODO: other formats (BGRA8 is the most common)
}

#[derive(Debug)]
pub struct Frame {
    /// Timestamp in microseconds since unix epoch
    pub timestamp: i64,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: Format,
}

#[derive(Error, Debug)]
pub enum Error {
    XCap(#[from] XCapError),

    NoMonitorDetected,

    #[cfg(target_os = "linux")]
    WaylandScreenCopy(#[from] wayland::WaylandCaptureError),

    #[cfg(target_os = "linux")]
    All(Vec<Self>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::XCap(error) => write!(f, "XCap error: {error}"),

            Error::NoMonitorDetected => f.write_str("No monitor detected"),

            #[cfg(target_os = "linux")]
            Error::WaylandScreenCopy(error) => write!(f, "Wayland screencopy error: {error}"),

            #[cfg(target_os = "linux")]
            Error::All(errors) => {
                f.write_str("All options failed. Errors: [")?;
                if !errors.is_empty() {
                    errors[0].fmt(f)?;
                    for err in &errors[1..] {
                        f.write_str(", ")?;
                        err.fmt(f)?;
                    }
                }
                f.write_str("]")
            }
        }
    }
}

/// Cross-platform function that works on most setups.
pub fn capture_video(frame_rate: u64) -> Result<impl Iterator<Item = Frame>, Error> {
    #[cfg(not(target_os = "linux"))]
    return crate::xcap::capture_video(frame_rate);

    #[cfg(target_os = "linux")]
    {
        let xcap_error = match crate::xcap::capture_video(frame_rate) {
            Ok(video) => return Ok(Box::new(video) as Box<dyn Iterator<Item = Frame> + Send>),
            Err(err) => err,
        };
        let wayland_error = match crate::wayland::capture_video(frame_rate) {
            Ok(video) => return Ok(Box::new(video) as Box<dyn Iterator<Item = Frame> + Send>),
            Err(err) => err,
        };
        Err(Error::All(vec![xcap_error, wayland_error.into()]))
    }
}

/// Fallback function that works on Linux Wayland devices with wlr-screencopy-unstable-v1.
#[cfg(target_os = "linux")]
pub use crate::wayland::capture_video as capture_video_wayland;

fn subsample_video<T>(
    mut video: impl Iterator<Item = T> + Send,
    frame_rate: u64,
) -> impl Iterator<Item = T> + Send {
    let frame_duration = Duration::from_nanos(1_000_000_000 / frame_rate);
    let mut last = Instant::now();

    std::iter::from_fn(move || {
        let now_frame = Instant::now();
        debug!(
            "Time since last frame: {:.2}ms",
            (now_frame - last).as_micros() as f32 / 1000.0
        );
        let diff = now_frame.duration_since(last);
        if diff < frame_duration {
            trace!(
                "Sleeping {:.2}ms",
                (frame_duration - diff).as_micros() as f32 / 1000.0
            );
            // TODO: sleep tends to overshoot by ~50 micros, should this be compensated for?
            std::thread::sleep(frame_duration - diff);
        }
        let now_capture = Instant::now();
        video.next().inspect(|_| {
            debug!(
                "Frame capture time: {:.2}ms",
                (Instant::now() - now_capture).as_micros() as f32 / 1000.0
            );
            last = now_frame;
        })
    })
}
