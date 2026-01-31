#![windows_subsystem = "windows"]

use std::collections::VecDeque;
use std::net::IpAddr;
use std::thread::sleep;
use std::time::{Duration, Instant};

use notify_rust::{Notification, Timeout};
use winping::{Buffer, Pinger};

const SAMPLE_PERIOD: Duration = Duration::from_millis(250);
const LOSS_WINDOW: Duration = Duration::from_secs(30);

const ERROR_COOLDOWN: Duration = Duration::from_secs(30);
const LOSS_COOLDOWN: Duration = Duration::from_secs(30);
const LATENCY_COOLDOWN: Duration = Duration::from_secs(30);

const HIGH_LATENCY_MS: u32 = 100;
const LOSS_THRESHOLD_PCT: u32 = 10;
const MIN_SAMPLES_FOR_LOSS: usize = 10;

fn init_ip() -> IpAddr {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8.8.8.8".to_string())
        .parse::<IpAddr>()
        .expect("Could not parse IP Address")
}

fn show_notification(msg: &str) {
    let _ = Notification::new()
        .summary("NET TRACER")
        .body(msg)
        .timeout(Timeout::Milliseconds(5000))
        .show();
}

#[derive(Clone, Copy)]
struct Sample {
    t: Instant,
    lost: bool,
}

fn main() {
    show_notification("NET TRACER running");

    let dst = init_ip();
    let pinger = Pinger::new().expect("Failed to create pinger");
    let mut buffer = Buffer::new();

    let mut samples: VecDeque<Sample> = VecDeque::new();
    let mut lost_in_window: u32 = 0;

    let mut next_error_notify_at = Instant::now();
    let mut next_loss_notify_at = Instant::now();
    let mut next_latency_notify_at = Instant::now();

    loop {
        let now = Instant::now();

        let ping_result = pinger.send(dst, &mut buffer);
        let lost = ping_result.is_err();

        samples.push_back(Sample { t: now, lost });
        if lost {
            lost_in_window += 1;
        }

        while let Some(front) = samples.front() {
            if now.duration_since(front.t) <= LOSS_WINDOW {
                break;
            }
            let old = samples.pop_front().unwrap();
            if old.lost {
                lost_in_window = lost_in_window.saturating_sub(1);
            }
        }

        if let Ok(ms) = ping_result {
            if ms > HIGH_LATENCY_MS && now >= next_latency_notify_at {
                show_notification(&format!("High latency: {ms} ms"));
                next_latency_notify_at = now + LATENCY_COOLDOWN;
            }
        } else if now >= next_error_notify_at {
            show_notification(&format!("Ping error: {}", ping_result.unwrap_err()));
            next_error_notify_at = now + ERROR_COOLDOWN;
        }

        let n = samples.len();
        if n >= MIN_SAMPLES_FOR_LOSS {
            let n_u32 = n as u32;
            let loss_pct = lost_in_window.saturating_mul(100) / n_u32;

            if loss_pct > LOSS_THRESHOLD_PCT && now >= next_loss_notify_at {
                show_notification(&format!(
                    "Packet loss last {}s: {}% ({} / {})",
                    LOSS_WINDOW.as_secs(),
                    loss_pct,
                    lost_in_window,
                    n_u32
                ));
                next_loss_notify_at = now + LOSS_COOLDOWN;
            }
        }

        sleep(SAMPLE_PERIOD);
    }
}
