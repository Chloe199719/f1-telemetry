//! A UDP protocol fixture stream for the local "Data Output from F1 25 v3.pdf".
//! Reader code is intentionally independent of these encoders.

#[path = "simulator/packets.rs"]
mod packets;

use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use clap::Parser;
use packets::{CAR_COUNT, EVENT_CODES, Frame, RUNNING_EVENTS};
use tokio::{net::UdpSocket, time::MissedTickBehavior};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Send all F1 25 v3 packet types and events as binary UDP fixtures"
)]
struct Args {
    /// Destination IP address and port (IPv6 must use [address]:port)
    #[arg(long, default_value = "127.0.0.1:20777")]
    target: SocketAddr,

    /// Rate for motion, lap data, telemetry, status, and Motion Ex packets
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u16).range(1..=240))]
    hz: u16,

    /// Stop after this many seconds; otherwise run until Ctrl+C
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    duration: Option<u32>,

    /// Seconds per synthetic lap, 1-3600 (short by default to exercise history quickly)
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u16).range(1..=3600))]
    lap_seconds: u16,

    /// Comma-separated packet IDs to emit (0-15); all types are enabled by default
    #[arg(long, value_delimiter = ',', default_value = "0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15", value_parser = clap::value_parser!(u8).range(0..=15))]
    packets: Vec<u8>,

    /// Milliseconds between event fixtures; the 15 running events repeat in order
    #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u32).range(1..))]
    event_interval_ms: u32,
}

fn period(id: usize, hz: u16) -> Duration {
    match id {
        1 | 5 | 9 => Duration::from_millis(500),
        4 => Duration::from_secs(5),
        10 => Duration::from_millis(100),
        11 | 12 => Duration::from_millis(50),
        14 | 15 => Duration::from_secs(1),
        _ => Duration::from_secs_f64(1.0 / f64::from(hz)),
    }
}

fn next_deadline(elapsed: Duration, period: Duration) -> Duration {
    // Keep the schedule on its original grid; skip missed sends without a burst.
    let remainder = Duration::from_nanos((elapsed.as_nanos() % period.as_nanos()) as u64);
    elapsed + period - remainder
}

async fn send(socket: &UdpSocket, target: SocketAddr, packet: &[u8]) -> Result<()> {
    socket
        .send_to(packet, target)
        .await
        .with_context(|| format!("could not send packet ID {} to {target}", packet[6]))?;
    if packet[6] == 3 {
        info!(event = %String::from_utf8_lossy(&packet[29..33]), "sent event fixture");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let mut selected = [false; 16];
    for id in &args.packets {
        selected[usize::from(*id)] = true;
    }
    let bind_address = if args.target.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(bind_address)
        .await
        .context("could not bind simulator UDP socket")?;
    let session_uid = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos() as u64;
    let mut frame = Frame {
        session_uid,
        number: 0,
        overall_number: 0,
        time: 0.0,
    };
    // Fixed-rate packets still run at up to 20 Hz even when --hz is smaller.
    let mut ticker =
        tokio::time::interval(Duration::from_secs_f64(1.0 / f64::from(args.hz.max(20))));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_secs(u64::from(args.duration.unwrap_or(u32::MAX)));
    let event_period = Duration::from_millis(u64::from(args.event_interval_ms));
    let mut next_event = event_period;
    let mut event_index = 0;
    let mut next_packet = [Duration::ZERO; 16];
    let mut history_car = 0;
    let mut tyre_car = 0;
    let mut packet_count = 0_u64;

    info!(target = %args.target, hz = args.hz, event_codes = EVENT_CODES.len(), "F1 25 protocol fixtures; press Ctrl+C to stop");
    if selected[3] {
        for code in [b"SSTA", b"STLG", b"LGOT", b"DRSE"] {
            send(
                &socket,
                args.target,
                &packets::event(frame, code, args.lap_seconds),
            )
            .await?;
            packet_count += 1;
        }
    }

    loop {
        tokio::select! {
            biased;
            result = &mut shutdown => {
                result.context("could not listen for Ctrl+C")?;
                break;
            }
            _ = tokio::time::sleep_until(deadline), if args.duration.is_some() => break,
            _ = ticker.tick() => {}
        }
        let elapsed = started.elapsed();
        frame.time = elapsed.as_secs_f32();
        if selected[3] && elapsed >= next_event {
            let code = &RUNNING_EVENTS[event_index];
            send(
                &socket,
                args.target,
                &packets::event(frame, code, args.lap_seconds),
            )
            .await?;
            packet_count += 1;
            event_index = (event_index + 1) % RUNNING_EVENTS.len();
            next_event = next_deadline(elapsed, event_period);
        }
        for id in 0..16 {
            if !selected[id] || id == 3 || id == 8 || elapsed < next_packet[id] {
                continue;
            }
            let car = if id == 12 { tyre_car } else { history_car };
            send(
                &socket,
                args.target,
                &packets::data(id as u8, frame, args.lap_seconds, car, 0),
            )
            .await?;
            packet_count += 1;
            if id == 11 {
                history_car = (history_car + 1) % CAR_COUNT as u8;
            }
            if id == 12 {
                tyre_car = (tyre_car + 1) % CAR_COUNT as u8;
            }
            if id == 15 && packets::completed_laps(frame, args.lap_seconds) >= 50 {
                send(
                    &socket,
                    args.target,
                    &packets::data(15, frame, args.lap_seconds, 0, 50),
                )
                .await?;
                packet_count += 1;
            }
            next_packet[id] = next_deadline(elapsed, period(id, args.hz));
        }
        frame.number = frame.number.wrapping_add(1);
        frame.overall_number = frame.overall_number.wrapping_add(1);
    }

    frame.time = started.elapsed().as_secs_f32();
    if selected[3] {
        for code in [b"CHQF", b"RCWN"] {
            send(
                &socket,
                args.target,
                &packets::event(frame, code, args.lap_seconds),
            )
            .await?;
            packet_count += 1;
        }
    }
    if selected[8] {
        send(
            &socket,
            args.target,
            &packets::data(8, frame, args.lap_seconds, 0, 0),
        )
        .await?;
        packet_count += 1;
    }
    if selected[11] {
        for car in 0..CAR_COUNT as u8 {
            send(
                &socket,
                args.target,
                &packets::data(11, frame, args.lap_seconds, car, 0),
            )
            .await?;
            packet_count += 1;
        }
    }
    if selected[3] {
        send(
            &socket,
            args.target,
            &packets::event(frame, b"SEND", args.lap_seconds),
        )
        .await?;
        packet_count += 1;
    }
    info!(packet_count, "simulator stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_filter_and_numeric_bounds_are_validated() {
        let args = Args::try_parse_from(["simulator", "--packets", "3,6", "--hz", "30"]).unwrap();
        assert_eq!(args.packets, [3, 6]);
        assert_eq!(args.hz, 30);
        for option in [
            ["--packets", "16"],
            ["--packets", "-1"],
            ["--packets", ""],
            ["--hz", "0"],
            ["--hz", "241"],
            ["--duration", "0"],
            ["--lap-seconds", "0"],
            ["--lap-seconds", "3601"],
            ["--event-interval-ms", "0"],
        ] {
            assert!(
                Args::try_parse_from(["simulator", option[0], option[1]]).is_err(),
                "{option:?}"
            );
        }
    }

    #[test]
    fn fixed_packet_rates_do_not_depend_on_the_high_frequency_rate() {
        assert_eq!(period(4, 60), Duration::from_secs(5));
        for id in [1, 5, 9] {
            assert_eq!(period(id, 60), Duration::from_millis(500));
        }
        assert_eq!(period(10, 60), Duration::from_millis(100));
        for id in [11, 12] {
            assert_eq!(period(id, 1), Duration::from_millis(50));
        }
        for id in [14, 15] {
            assert_eq!(period(id, 60), Duration::from_secs(1));
        }
        for id in [0, 2, 6, 7, 13] {
            assert_eq!(period(id, 20), Duration::from_millis(50));
        }
    }

    #[test]
    fn deadlines_stay_on_the_grid_when_a_tick_is_late() {
        let interval = Duration::from_millis(50);
        assert_eq!(next_deadline(Duration::ZERO, interval), interval);
        assert_eq!(
            next_deadline(Duration::from_millis(51), interval),
            Duration::from_millis(100)
        );
        assert_eq!(
            next_deadline(Duration::from_millis(350), interval),
            Duration::from_millis(400)
        );
    }
}
