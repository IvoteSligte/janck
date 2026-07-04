#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::capture_video;

#[cfg(not(target_os = "linux"))]
use windows_and_macos_are_not_yet_supported;

pub struct Rgb8Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
