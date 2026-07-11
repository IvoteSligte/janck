// NOTE: the linux wayland implementation is based on https://github.com/sunsided/miniscreenshot/miniscreenshot-wayland
// but there are also other implementations there that could be useful (miniscreenshot-x11, miniscreenshot-portal)

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use target_os_is_not_supported;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bgra8,
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

pub fn capture_video(frame_rate: u64) -> impl Iterator<Item = Frame> {
    #[cfg(target_os = "linux")]
    return linux::capture_video(frame_rate);
    
    #[cfg(target_os = "windows")]
    return windows::capture_video(frame_rate);
    
    #[cfg(target_os = "macos")]
    return macos::capture_video(frame_rate);
}
