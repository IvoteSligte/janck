use std::time::{Duration, Instant};

const SECS: u64 = 3;

fn main() {
    env_logger::init();
    let start = Instant::now();
    let mut i = 0;
    for _ in janck::capture_video(60).unwrap() {
        if (Instant::now() - start) >= Duration::from_secs(SECS) {
            break;
        }
        i += 1;
    }
    println!(
        "{i} frames captured in approximately {SECS} seconds ({} fps)",
        i as f32 / SECS as f32
    );
}
