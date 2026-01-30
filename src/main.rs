#![windows_subsystem = "windows"]

use std::net::IpAddr;
use std::thread::sleep;
use std::time::{Duration, Instant};

use notify_rust::{Notification, Timeout};
use winping::{Buffer, Pinger};

fn init_ip() -> IpAddr {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8.8.8.8".to_string())
        .parse::<IpAddr>()
        .expect("Could not parse IP Address")
}

fn ping_ip(pinger: &Pinger, dst: IpAddr, buffer: &mut Buffer) -> Result<u32, String> {
    pinger.send(dst, buffer).map_err(|e| e.to_string())
}

fn show_notification(msg: &str) -> Result<(), Box<dyn std::error::Error>> {
    Notification::new()
        .summary("NET TRACER")
        .body(msg)
        .timeout(Timeout::Milliseconds(5000))
        .show()?;
    Ok(())
}

fn main() {
    show_notification("NET TRACER running").ok();

    let dst = init_ip();
    let pinger = Pinger::new().expect("Failed to create pinger");
    let mut buffer = Buffer::new();

    let mut last_notify = Instant::now() - Duration::from_secs(60);

    loop {
        match ping_ip(&pinger, dst, &mut buffer) {
            Ok(ms) => {
                if ms > 100 && last_notify.elapsed() > Duration::from_secs(30) {
                    let body = format!("High latency: {} ms", ms);
                    show_notification(&body).ok();
                    last_notify = Instant::now();
                }
            }
            Err(e) => {
                if last_notify.elapsed() > Duration::from_secs(30) {
                    let body = format!("Ping error: {}", e);
                    show_notification(&body).ok();
                    last_notify = Instant::now();
                }
            }
        }

        sleep(Duration::from_millis(250));
    }
}
