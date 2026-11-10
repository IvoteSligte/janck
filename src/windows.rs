use log::{debug, info};
use windows_capture::d3d11::StagingTexture;
use windows_capture::dxgi_duplication_api::{DxgiDuplicationApi, DxgiDuplicationFormat};
use windows_capture::monitor::Monitor;

use crate::{Error, Frame, FrameInfo};

const TIMEOUT_MS: u32 = 1000;

fn next_frame(
    dxgi_duplication: &mut DxgiDuplicationApi,
    staging: &mut Option<StagingTexture>,
) -> Result<Frame, Error> {
    let mut frame = loop {
        let frame = dxgi_duplication.acquire_next_frame(TIMEOUT_MS)?;

        // Note: Windows-capture may send an empty frame, which we can check
        // by seeing if frame.frame_info().LastPresentTime is zero.
        if frame.frame_info().LastPresentTime == 0 {
            debug!("Skipped empty frame.");
            continue;
        }
        break frame;
    };
    let format = match frame.format() {
        DxgiDuplicationFormat::Bgra8 => crate::Format::Bgra8,
        DxgiDuplicationFormat::Rgba8 => crate::Format::Rgba8,
        _ => unreachable!(),
    };
    if staging.as_ref().is_none_or(|staging: &StagingTexture| {
        let s = staging.desc();
        let f = frame.texture_desc();
        s.Width != f.Width || s.Height != f.Height || s.Format != f.Format
    }) {
        *staging = Some(StagingTexture::new(
            frame.device(),
            frame.width(),
            frame.height(),
            frame.texture_desc().Format,
        )?);
    }
    let staging = staging.as_mut().unwrap();
    let mut buffer = frame.buffer_with(staging)?;
    Ok(Frame {
        bytes: buffer.as_raw_buffer().to_vec(),
        info: FrameInfo {
            width: buffer.width(),
            height: buffer.height(),
            stride: buffer.row_pitch(),
            format,
        },
    })
}

/// Uses `DXGI` to capture video.
pub fn capture_video(
    frame_rate: u32,
) -> Result<impl Iterator<Item = Result<Frame, crate::Error>> + Send, crate::Error> {
    let monitor = Monitor::primary()?;
    let mut dxgi_duplication = DxgiDuplicationApi::new_options(
        monitor,
        &[DxgiDuplicationFormat::Bgra8, DxgiDuplicationFormat::Rgba8],
    )?;
    let mut staging = None;
    info!("Created DXGI Duplication object");

    Ok(std::iter::from_fn(move || {
        Some(next_frame(&mut dxgi_duplication, &mut staging))
    }))
}
