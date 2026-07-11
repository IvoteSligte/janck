//! Adapted from https://github.com/sunsided/miniscreenshot/tree/main/miniscreenshot-wayland
//!
//! Wayland screen capture using the **wlr-screencopy-v1** protocol.
//!
//! Supports wlroots-based compositors such as [Sway] and [Hyprland].
//! On compositors that do not implement `zwlr_screencopy_manager_v1` this
//! crate returns [`WaylandCaptureError::NoScreencopyManager`]. If the
//! compositor does not advertise `wl_shm`, [`WaylandCaptureError::NoShm`] is
//! returned instead.
//!
//! Both `wayland-client` and `wayland-protocols-wlr` are re-exported so
//! downstream consumers need not worry about version conflicts.
//!
//! [Sway]: https://swaywm.org/
//! [Hyprland]: https://hyprland.org/
//!
//! # Example
//!
//! ```rust,no_run
//! use miniscreenshot_wayland::WaylandCapture;
//!
//! let mut capture = WaylandCapture::connect().expect("connect to Wayland");
//! let shot = capture.capture_output(0).expect("capture first output");
//! shot.save("screenshot.png").unwrap();
//! ```

use log::trace;
pub use wayland_client;
pub use wayland_protocols_wlr;

use memmap2::MmapMut;
use std::{os::unix::io::AsFd, time::Instant};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
    protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool},
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum WaylandCaptureError {
    Connection(wayland_client::ConnectError),
    NoScreencopyManager,
    NoShm,
    OutputNotFound(usize),
    CaptureFailed,
    UnsupportedFormat(wl_shm::Format),
    Dispatch(wayland_client::DispatchError),
    Io(std::io::Error),
}

impl std::fmt::Display for WaylandCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "Wayland connection error: {e}"),
            Self::NoScreencopyManager => write!(f, "compositor lacks zwlr_screencopy_manager_v1"),
            Self::NoShm => write!(f, "compositor lacks wl_shm"),
            Self::OutputNotFound(i) => write!(f, "output index {i} not found"),
            Self::CaptureFailed => write!(f, "compositor reported capture failure"),
            Self::UnsupportedFormat(format) => write!(f, "unsupported capture format: {format:?}"),
            Self::Dispatch(e) => write!(f, "Wayland dispatch error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for WaylandCaptureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connection(e) => Some(e),
            Self::Dispatch(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

// ── Internal state ────────────────────────────────────────────────────────────

#[derive(Default)]
struct AppState {
    screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
    shm: Option<wl_shm::WlShm>,
    outputs: Vec<wl_output::WlOutput>,
    frame: Option<FrameCapture>,
}

#[derive(Default)]
struct FrameCapture {
    shm_format: Option<wl_shm::Format>,
    width: u32,
    height: u32,
    stride: u32,
    buffer_done: bool,
    ready: bool,
    failed: bool,
    mmap: Option<MmapMut>,
    wl_buffer: Option<wl_buffer::WlBuffer>,
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "zwlr_screencopy_manager_v1" => {
                    state.screencopy_manager = Some(registry.bind(name, version.min(3), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "wl_output" => {
                    state
                        .outputs
                        .push(registry.bind(name, version.min(4), qh, ()));
                }
                _ => {}
            }
        }
    }
}

macro_rules! impl_dummy_dispatch {
    ($namespace:ident :: $item:ident) => {
        impl Dispatch<$namespace::$item, ()> for AppState {
            fn event(
                _: &mut Self,
                _: &$namespace::$item,
                _: $namespace::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

impl_dummy_dispatch!(wl_shm::WlShm);
impl_dummy_dispatch!(wl_shm_pool::WlShmPool);
impl_dummy_dispatch!(wl_buffer::WlBuffer);
impl_dummy_dispatch!(wl_output::WlOutput);
impl_dummy_dispatch!(zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1);

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(frame) = state.frame.as_mut() else {
            return;
        };
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format: WEnum::Value(fmt),
                width,
                height,
                stride,
            } if frame.shm_format.is_none() => {
                frame.shm_format = Some(fmt);
                frame.width = width;
                frame.height = height;
                frame.stride = stride;
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                frame.buffer_done = true;
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                frame.ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                frame.failed = true;
            }
            _ => {}
        }
    }
}

// ── SHM helper ────────────────────────────────────────────────────────────────

fn create_shm_file(size: usize) -> std::io::Result<std::fs::File> {
    use std::os::unix::io::FromRawFd;
    let name = b"miniscreenshot\0";
    let fd = unsafe {
        libc::syscall(
            libc::SYS_memfd_create,
            name.as_ptr(),
            libc::MFD_CLOEXEC as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: syscall returned a valid, owned file descriptor.
    let file = unsafe { std::fs::File::from_raw_fd(fd as libc::c_int) };
    file.set_len(size as u64)?;
    Ok(file)
}

// ── WaylandCapture ────────────────────────────────────────────────────────────

/// A Wayland screen-capture session backed by `zwlr_screencopy_manager_v1`.
pub struct WaylandCapture {
    event_queue: EventQueue<AppState>,
    state: AppState,
}

impl WaylandCapture {
    /// Connect to the Wayland compositor and bind required globals.
    pub fn connect() -> Result<Self, WaylandCaptureError> {
        let conn = Connection::connect_to_env().map_err(WaylandCaptureError::Connection)?;
        let mut event_queue = conn.new_event_queue::<AppState>();
        let qh = event_queue.handle();
        let mut state = AppState::default();

        conn.display().get_registry(&qh, ());
        event_queue
            .roundtrip(&mut state)
            .map_err(WaylandCaptureError::Dispatch)?;

        if state.screencopy_manager.is_none() {
            return Err(WaylandCaptureError::NoScreencopyManager);
        }
        if state.shm.is_none() {
            return Err(WaylandCaptureError::NoShm);
        }
        Ok(Self { event_queue, state })
    }

    /// Number of outputs (monitors) available.
    #[allow(unused)]
    pub fn output_count(&self) -> usize {
        self.state.outputs.len()
    }

    /// Capture the output at zero-based `output_index`.
    pub fn capture_output(
        &mut self,
        output_index: usize,
        overlay_cursor: bool,
    ) -> Result<crate::Frame, WaylandCaptureError> {
        let instant = Instant::now();
        let output = self
            .state
            .outputs
            .get(output_index)
            .ok_or(WaylandCaptureError::OutputNotFound(output_index))?
            .clone();

        let qh = self.event_queue.handle();
        let manager = self.state.screencopy_manager.as_ref().unwrap();

        self.state.frame = Some(FrameCapture::default());
        let frame = manager.capture_output(overlay_cursor as _, &output, &qh, ());

        trace!("Setup time: {}μs", (Instant::now() - instant).as_micros());
        let instant = Instant::now();

        // Phase 1: wait for buffer info + buffer_done.
        loop {
            self.event_queue
                .blocking_dispatch(&mut self.state)
                .map_err(WaylandCaptureError::Dispatch)?;
            match &self.state.frame {
                Some(f) if f.failed => {
                    frame.destroy();
                    self.state.frame = None;
                    return Err(WaylandCaptureError::CaptureFailed);
                }
                Some(f) if f.buffer_done => break,
                _ => {}
            }
        }

        trace!(
            "Wait-for-buffer_info and buffer_done time: {}μs",
            (Instant::now() - instant).as_micros()
        );
        let instant = Instant::now();

        // Allocate SHM.
        let fc = self.state.frame.as_mut().unwrap();
        let shm_format = fc.shm_format.ok_or(WaylandCaptureError::CaptureFailed)?;
        let (width, height, stride) = (fc.width, fc.height, fc.stride);
        let buf_size = (stride * height) as usize;

        let file = create_shm_file(buf_size).map_err(WaylandCaptureError::Io)?;
        // SAFETY: `file` is a valid, non-empty, writable anonymous file.
        let mmap = unsafe { MmapMut::map_mut(&file) }.map_err(WaylandCaptureError::Io)?;

        let shm = self.state.shm.as_ref().unwrap();
        let pool = shm.create_pool(file.as_fd(), buf_size as i32, &qh, ());
        let wl_buf = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            shm_format,
            &qh,
            (),
        );
        pool.destroy();

        let fc = self.state.frame.as_mut().unwrap();
        fc.mmap = Some(mmap);
        fc.wl_buffer = Some(wl_buf.clone());
        frame.copy(&wl_buf);

        trace!(
            "Allocate SHM time: {}μs",
            (Instant::now() - instant).as_micros()
        );
        let instant = Instant::now();

        // Phase 2: wait for ready or failed.
        loop {
            self.event_queue
                .blocking_dispatch(&mut self.state)
                .map_err(WaylandCaptureError::Dispatch)?;
            match &self.state.frame {
                Some(f) if f.failed => {
                    frame.destroy();
                    wl_buf.destroy();
                    self.state.frame = None;
                    return Err(WaylandCaptureError::CaptureFailed);
                }
                Some(f) if f.ready => break,
                _ => {}
            }
        }
        let timestamp = chrono::Utc::now().timestamp_micros();

        trace!(
            "Wait-for-ready or failed time: {}μs",
            (Instant::now() - instant).as_micros()
        );
        let instant = Instant::now();

        // Extract pixels.
        let fc = self.state.frame.take().unwrap();
        frame.destroy();
        wl_buf.destroy();

        trace!(
            "Extract pixels time: {}μs",
            (Instant::now() - instant).as_micros()
        );
        let mmap = fc.mmap.unwrap();
        Ok(crate::Frame {
            timestamp,
            bytes: mmap.to_vec(),
            width,
            height,
            stride,
            format: shm_format.try_into()?,
        })
    }
}

// ── Format conversion ──────────────────────────────────────────────────────────

impl TryFrom<wl_shm::Format> for crate::Format {
    type Error = WaylandCaptureError;

    fn try_from(value: wl_shm::Format) -> Result<Self, Self::Error> {
        match value {
            // ARGB/XRGB map to BGRA in little-endian u32 format as the bytes are reversed, which is what most CPUs use
            // TODO: big-endian conversions
            wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => Ok(crate::Format::Bgra8),
            _ => Err(WaylandCaptureError::UnsupportedFormat(value)),
        }
    }
}

// ── API ────────────────────────────────────────────────────────────────────────

const SHOW_CURSOR: bool = true;

pub fn capture_video(
    frame_rate: u64,
) -> Result<impl Iterator<Item = crate::Frame>, WaylandCaptureError> {
    let mut capture = WaylandCapture::connect()?;
    let video = std::iter::from_fn(move || {
        // TODO: select primary output (if that is not already output 0)
        capture.capture_output(0, SHOW_CURSOR).ok()
    });
    Ok(crate::subsample_video(video, frame_rate))
}
