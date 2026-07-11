use xcap::Monitor;

use crate::Error;

fn get_primary_monitor() -> Result<Monitor, Error> {
    let monitors = Monitor::all()?;
    let mut error = None;
    for m in monitors {
        match m.is_primary() {
            Ok(true) => return Ok(m),
            Ok(false) => continue,
            Err(err) => error = Some(err),
        }
    }
    Err(error.map(Error::from).unwrap_or(Error::NoMonitorDetected))
}

pub fn capture_video(frame_rate: u64) -> Result<impl Iterator<Item = crate::Frame>, Error> {
    let monitor = get_primary_monitor()?;
    let (recorder, receiver) = monitor.video_recorder()?;

    recorder.start()?;
    let video = receiver.into_iter().map(|xcap_frame| crate::Frame {
        timestamp: chrono::Utc::now().timestamp_micros(),
        bytes: xcap_frame.raw,
        width: xcap_frame.width,
        height: xcap_frame.height,
        stride: xcap_frame.width * 4,
        format: crate::Format::Bgra8,
    });
    Ok(crate::subsample_video(video, frame_rate))
}
