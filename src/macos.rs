use crate::Frame;

pub fn capture_video(frame_rate: u64) -> Result<impl Iterator<Item = Frame> + Send, ()> {
    unimplemented!();
    Ok(std::iter::empty())
}
