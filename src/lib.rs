// NOTE: the linux wayland implementation is based on https://github.com/sunsided/miniscreenshot/miniscreenshot-wayland
// but there are also other implementations there that could be useful (miniscreenshot-x11, miniscreenshot-portal)

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::capture_video;

#[cfg(not(target_os = "linux"))]
use windows_and_macos_are_not_yet_supported;

pub mod image;
pub use image::{Yuv420Image, Rgb8Image};
