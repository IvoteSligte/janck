use std::time::{Duration, Instant};

const SECS: u64 = 30;

fn main() {
    env_logger::init();
    let start = Instant::now();
    let mut prev = start;
    let mut i = 0;
    for frame_result in janck::capture_video(u32::MAX).unwrap() {
        let frame = frame_result.unwrap();
        let now = Instant::now();
        println!(
            "frame: {}, dimensions: {}x{}, elapsed: {:?}, total elapsed: {:?}",
            i,
            frame.info.width,
            frame.info.height,
            now - prev,
            now - start,
        );
        prev = now;
        if now - start >= Duration::from_secs(SECS) {
            break;
        }
        i += 1;
    }
    println!(
        "{i} frames captured in approximately {SECS} seconds ({} fps)",
        i as f32 / SECS as f32
    );
}
