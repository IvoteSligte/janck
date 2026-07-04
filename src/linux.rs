use miniscreenshot_wayland::WaylandCapture;

use crate::Rgb8Image;

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

impl Iterator for State {
    type Item = Rgb8Image;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.capture())
    }
}

pub fn capture_video() -> impl Iterator<Item = Rgb8Image> {
    State::new()
}
