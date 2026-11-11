// NOTE: the linux wayland implementation is based on https://github.com/sunsided/miniscreenshot/miniscreenshot-wayland
// but there are also other implementations there that could be useful (miniscreenshot-x11, miniscreenshot-portal)

use log::{info, warn};
use thiserror::Error;

#[cfg(target_os = "linux")]
pub mod wayland;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bgra8,
    Rgba8,
    // TODO: other formats
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: Format,
}

#[derive(Debug)]
pub struct Frame {
    pub bytes: Vec<u8>,
    pub info: FrameInfo,
}

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(target_os = "linux")]
    #[error("Wayland error: {0}")]
    Wayland(#[from] wayland::WaylandCaptureError),
}

pub fn capture_video(
    frame_rate: u32,
) -> Result<impl Iterator<Item = Result<Frame, Error>> + Send, Error> {
    info!("Trying Wayland screencopy");
    Ok(crate::wayland::capture_video(frame_rate)
        .inspect_err(|err| {
            warn!("Wayland screencopy failed: {err}");
            info!("X11 is not supported");
        })?
        .map(|result| result.map_err(Error::from)))
}
