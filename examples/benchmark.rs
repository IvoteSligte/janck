use std::time::{Duration, Instant};

const SECS: u64 = 3;

fn main() {
    let start = Instant::now();
    let mut i = 0;
    for _ in janck::capture_video(u64::MAX) {
        if (Instant::now() - start) >= Duration::from_secs(SECS) {
            break;
        }
        i += 1;
    }
    println!(
        "{i} frames captured in {SECS} seconds ({} fps)",
        i as f32 / SECS as f32
    );
}
