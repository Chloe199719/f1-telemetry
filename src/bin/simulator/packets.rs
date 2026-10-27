//! Packed, little-endian fixtures for every packet in the local F1 25 v3 PDF.
//! These encoders deliberately do not depend on the reader's types or decoder.

use std::f32::consts::TAU;

use bytes::{BufMut, BytesMut};

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

pub const HEADER_SIZE: usize = 29;
pub const CAR_COUNT: usize = 22;
pub const PACKET_SIZES: [usize; 16] = [
    1349, 753, 1285, 45, 1284, 1133, 1352, 1239, 1042, 954, 1041, 1460, 231, 273, 101, 1131,
];
const TRACK_LENGTH: f32 = 5891.0;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub session_uid: u64,
    pub number: u32,
    pub overall_number: u32,
    pub time: f32,
}

impl Frame {
    pub fn header(self, packet_id: u8) -> BytesMut {
        let mut packet = BytesMut::with_capacity(PACKET_SIZES[usize::from(packet_id)]);
        packet.put_u16_le(2025); // m_packetFormat
        packet.put_u8(25); // m_gameYear
        packet.put_u8(1); // m_gameMajorVersion
        packet.put_u8(0); // m_gameMinorVersion
        packet.put_u8(1); // m_packetVersion (all v3 packet types use version 1)
        packet.put_u8(packet_id); // m_packetId
        packet.put_u64_le(self.session_uid); // m_sessionUID
        packet.put_f32_le(self.time); // m_sessionTime
        packet.put_u32_le(self.number); // m_frameIdentifier
        packet.put_u32_le(self.overall_number); // m_overallFrameIdentifier
        packet.put_u8(0); // m_playerCarIndex
        packet.put_u8(255); // m_secondaryPlayerCarIndex
        assert_eq!(packet.len(), HEADER_SIZE);
        packet
    }
}

/// Encode a complete data packet; ID 3 is encoded separately by `event`.
pub fn data(id: u8, frame: Frame, lap_seconds: u16, car_index: u8, lap_start: u8) -> BytesMut {
    let mut packet = frame.header(id);
    match id {
        0 => motion(&mut packet, frame, lap_seconds),
        1 => session(&mut packet, frame),
        2 => lap_data(&mut packet, frame, lap_seconds),
        4 => participants(&mut packet),
        5 => setups(&mut packet),
        6 => telemetry(&mut packet, frame, lap_seconds),
        7 => status(&mut packet, frame, lap_seconds),
        8 => classification(&mut packet, frame, lap_seconds),
        9 => lobby(&mut packet),
        10 => damage(&mut packet, frame, lap_seconds),
        11 => history(&mut packet, frame, lap_seconds, car_index),
        12 => tyre_sets(&mut packet, frame, lap_seconds, car_index),
        13 => motion_ex(&mut packet, frame, lap_seconds),
        14 => time_trial(&mut packet, lap_seconds),
        15 => lap_positions(&mut packet, frame, lap_seconds, lap_start),
        _ => panic!("packet ID {id} is not a data packet"),
    }
    // Never pad whole packets: a missing field must fail, including in release builds.
    assert_eq!(
        packet.len(),
        PACKET_SIZES[usize::from(id)],
        "packet ID {id}"
    );
    packet
}

pub fn completed_laps(frame: Frame, lap_seconds: u16) -> u8 {
    (frame.time / f32::from(lap_seconds)).floor().min(99.0) as u8
}

fn progress(frame: Frame, lap_seconds: u16) -> f32 {
    frame.time.rem_euclid(f32::from(lap_seconds)) / f32::from(lap_seconds)
}

fn current_lap_ms(frame: Frame, lap_seconds: u16) -> u32 {
    (frame.time.rem_euclid(f32::from(lap_seconds)) * 1000.0).round() as u32
}

fn floats(packet: &mut BytesMut, values: impl IntoIterator<Item = f32>) {
    for value in values {
        packet.put_f32_le(value);
    }
}

fn name(packet: &mut BytesMut, car: u8) {
    let value = format!("Test Drivér {:02}", car + 1);
    let mut field = [0; 32];
    field[..value.len()].copy_from_slice(value.as_bytes());
    packet.extend_from_slice(&field);
}

fn team(car: u8) -> u8 {
    if car < 20 { car / 2 } else { 104 }
}

fn sector_time(packet: &mut BytesMut, milliseconds: u32) {
    packet.put_u16_le((milliseconds % 60_000) as u16);
    packet.put_u8((milliseconds / 60_000) as u8);
}

fn motion(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    for car in 0..CAR_COUNT {
        let angle = progress(frame, lap_seconds) * TAU + car as f32 * 0.17;
        let speed = 60.0 + car as f32;
        floats(packet, [angle.cos() * 500.0, 2.0, angle.sin() * 500.0]); // World position
        floats(packet, [-angle.sin() * speed, 0.0, angle.cos() * speed]); // World velocity
        for direction in [
            -angle.sin(),
            0.0,
            angle.cos(),
            angle.cos(),
            0.0,
            angle.sin(),
        ] {
            packet.put_i16_le((direction * 32767.0) as i16); // Forward and right unit vectors
        }
        floats(packet, [angle.sin() * 2.0, angle.cos(), 1.0]); // Lateral, longitudinal, vertical G
        floats(packet, [angle, 0.01, -0.02]); // Yaw, pitch, roll
    }
}

fn session(packet: &mut BytesMut, frame: Frame) {
    packet.put_u8(0); // Weather: clear
    packet.put_i8(35); // Track temperature
    packet.put_i8(24); // Air temperature
    packet.put_u8(100); // Total laps
    packet.put_u16_le(TRACK_LENGTH as u16);
    packet.put_u8(15); // Session type: race
    packet.put_i8(7); // Silverstone
    packet.put_u8(0); // F1 Modern
    packet.put_u16_le(3600_u16.saturating_sub(frame.time as u16)); // Time left
    packet.put_u16_le(3600); // Session duration
    packet.put_u8(80); // Pit speed limit
    packet.put_u8(0); // Paused
    packet.put_u8(0); // Spectating
    packet.put_u8(255); // Spectator car
    packet.put_u8(0); // SLI Pro support
    packet.put_u8(3); // Number of marshal zones
    for zone in 0..21 {
        packet.put_f32_le(if zone < 3 { zone as f32 / 3.0 } else { 0.0 });
        packet.put_i8(0); // Zone flag: none
    }
    packet.put_u8(0); // Safety car status
    packet.put_u8(0); // Offline
    packet.put_u8(3); // Weather forecast samples
    for sample in 0..64 {
        if sample < 3 {
            packet.extend_from_slice(&[15, sample * 5, 0, 35, 2, 24, 2, 0]);
        } else {
            packet.extend_from_slice(&[0; 8]); // Unused forecast entry
        }
    }
    packet.put_u8(0); // Forecast accuracy
    packet.put_u8(85); // AI difficulty
    packet.put_u32_le(101); // Season link
    packet.put_u32_le(102); // Weekend link
    packet.put_u32_le(103); // Session link
    packet.put_u8(5); // Ideal pit lap
    packet.put_u8(7); // Latest pit lap
    packet.put_u8(3); // Rejoin position
    packet.put_u8(0); // Steering assist
    packet.put_u8(0); // Braking assist
    packet.put_u8(1); // Manual gearbox
    packet.put_u8(0); // Pit assist
    packet.put_u8(0); // Pit release assist
    packet.put_u8(0); // ERS assist
    packet.put_u8(0); // DRS assist
    packet.put_u8(1); // Dynamic racing line: corners
    packet.put_u8(1); // 3D racing line
    packet.put_u8(4); // Grand Prix game mode
    packet.put_u8(1); // Race rules
    packet.put_u32_le(14 * 60); // Time of day in minutes
    packet.put_u8(7); // Full session length
    packet.put_u8(1); // Lead player: KPH
    packet.put_u8(0); // Lead player: Celsius
    packet.put_u8(1); // Secondary player: KPH
    packet.put_u8(0); // Secondary player: Celsius
    packet.put_u8(0); // Safety car periods
    packet.put_u8(0); // Virtual safety car periods
    packet.put_u8(0); // Red flag periods
    packet.put_u8(1); // Equal car performance
    packet.put_u8(1); // Recovery mode: flashbacks
    packet.put_u8(3); // Unlimited flashbacks
    packet.put_u8(1); // Realistic surface
    packet.put_u8(1); // Hard low fuel mode
    packet.put_u8(0); // Manual race starts
    packet.put_u8(1); // Surface and carcass tyre temperatures
    packet.put_u8(0); // Pit lane tyre simulation on
    packet.put_u8(2); // Standard car damage
    packet.put_u8(1); // Standard damage rate
    packet.put_u8(2); // Collisions on
    packet.put_u8(0); // Collisions off for first lap only
    packet.put_u8(0); // Unsafe pit release on
    packet.put_u8(0); // Griefing option off
    packet.put_u8(1); // Strict corner cutting
    packet.put_u8(1); // Parc ferme
    packet.put_u8(2); // Immersive pit stops
    packet.put_u8(2); // Standard safety cars
    packet.put_u8(1); // Immersive safety cars
    packet.put_u8(1); // Formation lap on
    packet.put_u8(1); // Immersive formation lap
    packet.put_u8(2); // Standard red flags
    packet.put_u8(0); // Solo licence level unaffected
    packet.put_u8(0); // Multiplayer licence level unaffected
    packet.put_u8(3); // Sessions in weekend
    packet.extend_from_slice(&[1, 5, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    floats(packet, [TRACK_LENGTH / 3.0, TRACK_LENGTH * 2.0 / 3.0]);
}

fn lap_data(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    let completed = completed_laps(frame, lap_seconds);
    let lap_ms = u32::from(lap_seconds) * 1000;
    let current_ms = current_lap_ms(frame, lap_seconds);
    let sector = (progress(frame, lap_seconds) * 3.0) as u8;
    for car in 0..CAR_COUNT as u8 {
        packet.put_u32_le(if completed > 0 { lap_ms } else { 0 });
        packet.put_u32_le(current_ms);
        sector_time(packet, if sector >= 1 { lap_ms / 3 } else { 0 });
        sector_time(packet, if sector >= 2 { lap_ms / 3 } else { 0 });
        sector_time(packet, if car > 0 { 250 } else { 0 }); // Delta to car in front
        sector_time(packet, u32::from(car) * 250); // Delta to leader
        floats(
            packet,
            [
                progress(frame, lap_seconds) * TRACK_LENGTH,
                (f32::from(completed) + progress(frame, lap_seconds)) * TRACK_LENGTH,
                0.0, // Safety car delta
            ],
        );
        packet.put_u8(car + 1); // Position
        packet.put_u8(completed + 1); // Current lap
        packet.put_u8(0); // Pit status
        packet.put_u8(0); // Pit stops
        packet.put_u8(sector);
        packet.put_u8(0); // Lap invalid
        packet.put_u8(0); // Penalties
        packet.put_u8(0); // Total warnings
        packet.put_u8(0); // Corner cutting warnings
        packet.put_u8(0); // Unserved drive-throughs
        packet.put_u8(0); // Unserved stop-go penalties
        packet.put_u8(car + 1); // Grid position
        packet.put_u8(4); // On track
        packet.put_u8(2); // Active
        packet.put_u8(0); // Pit timer active
        packet.put_u16_le(0); // Pit lane time
        packet.put_u16_le(0); // Pit stop time
        packet.put_u8(0); // Serve penalty at pit stop
        packet.put_f32_le(330.0 - f32::from(car)); // Fastest speed trap
        packet.put_u8(if completed > 0 { 1 } else { 255 }); // Fastest speed trap lap
    }
    packet.extend_from_slice(&[255, 255]); // No time-trial ghosts in the race fixture
}

fn participants(packet: &mut BytesMut) {
    packet.put_u8(CAR_COUNT as u8);
    for car in 0..CAR_COUNT as u8 {
        packet.put_u8(u8::from(car != 0)); // AI controlled
        packet.put_u8(if car == 0 { 255 } else { 9 }); // Human or AI driver ID
        packet.put_u8(car); // Network ID
        packet.put_u8(team(car));
        packet.put_u8(0); // My Team
        packet.put_u8(car + 1); // Race number
        packet.put_u8(10); // Nationality: British
        name(packet, car);
        packet.put_u8(1); // Public telemetry
        packet.put_u8(1); // Show online names
        packet.put_u16_le(100); // Tech level
        packet.put_u8(1); // Steam
        packet.put_u8(4); // Number of livery colours
        packet.extend_from_slice(&[200, car * 8, 32, 20, 40, 60, 240, 240, 240, 0, 0, 0]);
    }
}

fn setups(packet: &mut BytesMut) {
    for _ in 0..CAR_COUNT {
        packet.extend_from_slice(&[30, 28, 65, 50]); // Wings and differential
        floats(packet, [-2.5, -1.0, 0.05, 0.2]); // Camber and toe
        packet.extend_from_slice(&[30, 10, 8, 5, 20, 45, 100, 56, 50]);
        floats(packet, [21.5, 21.5, 23.0, 23.0]); // RL, RR, FL, FR pressure
        packet.put_u8(6); // Ballast
        packet.put_f32_le(50.0); // Fuel load
    }
    packet.put_f32_le(32.0); // Next front wing value
}

fn telemetry(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    for car in 0..CAR_COUNT {
        let phase = frame.time / f32::from(lap_seconds) * TAU + car as f32 * 0.17;
        let pace = (phase.sin() + 1.0) * 0.5;
        let throttle = phase.cos().max(0.0);
        let brake = (-phase.cos()).max(0.0) * 0.8;
        let speed = (90.0 + pace * 240.0) as u16;
        let gear = ((speed / 40) as u8).clamp(1, 8);
        let rev_percent = (pace * 100.0) as u8;
        let lit_leds = u32::from(rev_percent) * 15 / 100;
        packet.put_u16_le(speed);
        floats(packet, [throttle, (phase * 2.0).sin() * 0.6, brake]);
        packet.put_u8(0); // Clutch
        packet.put_i8(gear as i8);
        packet.put_u16_le((6500.0 + pace * 5500.0) as u16);
        packet.put_u8(u8::from(speed > 280 && brake == 0.0));
        packet.put_u8(rev_percent);
        packet.put_u16_le(((1_u32 << lit_leds) - 1) as u16);
        for wheel in 0..4 {
            packet.put_u16_le((350.0 + brake * 500.0) as u16 + wheel);
        }
        packet.extend_from_slice(&[85, 86, 87, 88]); // Tyre surface temperatures, RL/RR/FL/FR
        packet.extend_from_slice(&[90, 91, 92, 93]); // Inner temperatures
        packet.put_u16_le(105); // Engine temperature
        floats(
            packet,
            [
                21.5 + pace * 0.3,
                21.6 + pace * 0.3,
                23.0 + pace * 0.3,
                23.1 + pace * 0.3,
            ],
        );
        packet.extend_from_slice(&[0, 0, 1, 1]); // Rear wheels on tarmac; front on kerb
    }
    packet.extend_from_slice(&[255, 255, 0]); // MFD panels closed; no suggested gear
}

fn status(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    let fraction = progress(frame, lap_seconds);
    for car in 0..CAR_COUNT as u8 {
        packet.extend_from_slice(&[0, 0, 1, 56, 0]); // TC, ABS, fuel mix, brake bias, pit limiter
        floats(packet, [(50.0 - frame.time * 0.05).max(0.0), 110.0, 20.0]);
        packet.put_u16_le(15_000); // Max RPM
        packet.put_u16_le(4000); // Idle RPM
        packet.put_u8(8); // Max gears
        packet.put_u8(1); // DRS allowed
        packet.put_u16_le(0); // DRS activation distance
        packet.put_u8(22); // F1 25 C6 actual compound
        packet.put_u8(16); // Soft visual compound
        packet.put_u8(completed_laps(frame, lap_seconds));
        packet.put_i8(1); // FIA flag: green
        floats(
            packet,
            [550_000.0 + f32::from(car) * 100.0, 120_000.0, 4_000_000.0],
        );
        packet.put_u8(2); // ERS deploy mode: hotlap
        floats(
            packet,
            [
                fraction * 1_000_000.0,
                fraction * 500_000.0,
                fraction * 2_000_000.0,
            ],
        );
        packet.put_u8(0); // Network paused
    }
}

fn classification(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    const POINTS: [u8; 10] = [25, 18, 15, 12, 10, 8, 6, 4, 2, 1];
    let laps = completed_laps(frame, lap_seconds);
    packet.put_u8(CAR_COUNT as u8);
    for car in 0..CAR_COUNT as u8 {
        packet.put_u8(car + 1); // Position
        packet.put_u8(laps);
        packet.put_u8(car + 1); // Grid position
        packet.put_u8(POINTS.get(usize::from(car)).copied().unwrap_or(0));
        packet.put_u8(0); // Pit stops
        packet.put_u8(3); // Finished
        packet.put_u8(10); // F1 25 result reason: session simulated
        packet.put_u32_le(if laps > 0 {
            u32::from(lap_seconds) * 1000
        } else {
            0
        });
        packet.put_f64_le(f64::from(frame.time) + f64::from(car) * 0.25);
        packet.put_u8(0); // Penalty seconds
        packet.put_u8(0); // Penalty count
        packet.put_u8(1); // Tyre stint count
        packet.extend_from_slice(&[22, 0, 0, 0, 0, 0, 0, 0]); // Actual compounds
        packet.extend_from_slice(&[16, 0, 0, 0, 0, 0, 0, 0]); // Visual compounds
        packet.extend_from_slice(&[laps, 0, 0, 0, 0, 0, 0, 0]); // Stint end laps
    }
}

fn lobby(packet: &mut BytesMut) {
    packet.put_u8(CAR_COUNT as u8);
    for car in 0..CAR_COUNT as u8 {
        packet.put_u8(u8::from(car != 0));
        packet.put_u8(team(car));
        packet.put_u8(10); // Nationality
        packet.put_u8(1); // Steam
        name(packet, car);
        packet.put_u8(car + 1); // Car number
        packet.put_u8(1); // Public telemetry
        packet.put_u8(1); // Show online names
        packet.put_u16_le(100); // Tech level
        packet.put_u8(1); // Ready
    }
}

fn damage(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    let wear = f32::from(completed_laps(frame, lap_seconds)) * 0.5;
    for car in 0..CAR_COUNT as u8 {
        floats(packet, [wear + 1.0, wear + 2.0, wear + 3.0, wear + 4.0]);
        packet.extend_from_slice(&[1, 2, 3, 4]); // Tyre damage
        packet.extend_from_slice(&[5, 6, 7, 8]); // Brake damage
        packet.extend_from_slice(&[9, 10, 11, 12]); // F1 25 tyre blisters
        packet.extend_from_slice(&[1, 2, 3, 4, 5, 6]); // FL wing, FR wing, rear wing, floor, diffuser, sidepod
        packet.extend_from_slice(&[0, 0]); // DRS and ERS faults
        packet.put_u8(car); // Gearbox damage
        packet.put_u8(2); // Engine damage
        packet.extend_from_slice(&[3, 4, 5, 6, 7, 8]); // MGU-H, ES, CE, ICE, MGU-K, TC wear
        packet.extend_from_slice(&[0, 0]); // Engine blown and seized
    }
}

fn history(packet: &mut BytesMut, frame: Frame, lap_seconds: u16, car_index: u8) {
    assert!(car_index < CAR_COUNT as u8);
    let completed = completed_laps(frame, lap_seconds);
    let lap_ms = u32::from(lap_seconds) * 1000;
    packet.put_u8(car_index);
    packet.put_u8(completed + 1); // Includes current partial lap
    packet.put_u8(1); // Tyre stint count
    let best = u8::from(completed > 0);
    packet.extend_from_slice(&[best; 4]); // Best lap and best sector lap numbers
    for lap in 0..100 {
        let time = if lap < usize::from(completed) {
            lap_ms
        } else if lap == usize::from(completed) {
            current_lap_ms(frame, lap_seconds)
        } else {
            0
        };
        packet.put_u32_le(time);
        let first = time.min(lap_ms / 3);
        let second = time.saturating_sub(first).min(lap_ms / 3);
        sector_time(packet, first);
        sector_time(packet, second);
        sector_time(packet, time - first - second);
        packet.put_u8(if lap < usize::from(completed) {
            0x0f
        } else {
            0
        });
    }
    packet.extend_from_slice(&[255, 22, 16]); // Current stint, C6/soft
    for _ in 1..8 {
        packet.extend_from_slice(&[0; 3]); // Unused tyre stints
    }
}

fn tyre_sets(packet: &mut BytesMut, frame: Frame, lap_seconds: u16, car_index: u8) {
    assert!(car_index < CAR_COUNT as u8);
    packet.put_u8(car_index);
    for set in 0..20 {
        let (actual, visual) = match set {
            0..=3 => (22, 16),  // C6/soft
            4..=8 => (18, 17),  // C3/medium
            9..=12 => (20, 18), // C1/hard
            13..=16 => (7, 7),  // Intermediate
            _ => (8, 8),        // Wet
        };
        packet.put_u8(actual);
        packet.put_u8(visual);
        packet.put_u8(if set == 0 {
            completed_laps(frame, lap_seconds)
        } else {
            0
        });
        packet.put_u8(1); // Available
        packet.put_u8(15); // Recommended session: race
        packet.put_u8(20); // Remaining lifespan
        packet.put_u8(18); // Recommended usable life
        packet.put_i16_le(-125 * set as i16); // Signed lap delta in milliseconds
        packet.put_u8(u8::from(set == 0)); // Fitted
    }
    packet.put_u8(0); // Fitted index
}

fn motion_ex(packet: &mut BytesMut, frame: Frame, lap_seconds: u16) {
    let wave = (progress(frame, lap_seconds) * TAU).sin();
    for values in [
        [0.01, 0.02, 0.03, 0.04],         // Suspension position
        [wave * 0.1; 4],                  // Suspension velocity
        [wave * 0.2; 4],                  // Suspension acceleration
        [60.0, 60.1, 60.2, 60.3],         // Wheel speed
        [0.01, 0.02, 0.03, 0.04],         // Wheel slip ratio
        [-0.01, -0.02, 0.03, 0.04],       // Wheel slip angle
        [1000.0, 1100.0, 1200.0, 1300.0], // Wheel lateral force
        [2000.0, 2100.0, 2200.0, 2300.0], // Wheel longitudinal force
    ] {
        floats(packet, values);
    }
    packet.put_f32_le(0.3); // Centre of gravity height
    floats(packet, [1.0, 0.1, 60.0]); // Local velocity
    floats(packet, [0.01, 0.02, 0.03]); // Angular velocity
    floats(packet, [0.04, 0.05, 0.06]); // Angular acceleration
    packet.put_f32_le(wave * 0.2); // Front wheel angle
    floats(packet, [3000.0, 3100.0, 3200.0, 3300.0]); // Wheel vertical force
    floats(packet, [0.02, 0.04, 0.01, -0.01, 0.02]); // Aero heights, suspension roll, chassis yaw
    packet.put_f32_le(-0.03); // F1 25 chassis pitch
    floats(packet, [-0.04, -0.05, -0.06, -0.07]); // F1 25 wheel camber
    floats(packet, [0.001, 0.002, 0.003, 0.004]); // F1 25 wheel camber gain
}

fn time_trial(packet: &mut BytesMut, lap_seconds: u16) {
    for car in 0..3 {
        let lap_ms = u32::from(lap_seconds) * 1000 + u32::from(car) * 250;
        packet.put_u8(car);
        packet.put_u8(team(car));
        packet.put_u32_le(lap_ms);
        packet.put_u32_le(lap_ms / 3);
        packet.put_u32_le(lap_ms / 3);
        packet.put_u32_le(lap_ms - 2 * (lap_ms / 3));
        packet.extend_from_slice(&[0, 0, 0, 1, 1, 1]); // TC, gearbox, ABS, equal performance, custom setup, valid
    }
}

fn lap_positions(packet: &mut BytesMut, frame: Frame, lap_seconds: u16, lap_start: u8) {
    assert!(lap_start == 0 || lap_start == 50);
    let recorded = completed_laps(frame, lap_seconds) + 1;
    let count = recorded.saturating_sub(lap_start).min(50);
    packet.put_u8(count);
    packet.put_u8(lap_start);
    for lap in 0..50 {
        for car in 0..CAR_COUNT as u8 {
            packet.put_u8(if lap < count { car + 1 } else { 0 });
        }
    }
}

pub const EVENT_CODES: [[u8; 4]; 21] = [
    *b"SSTA", *b"SEND", *b"FTLP", *b"RTMT", *b"DRSE", *b"DRSD", *b"TMPT", *b"CHQF", *b"RCWN",
    *b"PENA", *b"SPTP", *b"STLG", *b"LGOT", *b"DTSV", *b"SGSV", *b"FLBK", *b"BUTN", *b"RDFL",
    *b"OVTK", *b"SCAR", *b"COLL",
];

pub const RUNNING_EVENTS: [[u8; 4]; 15] = [
    *b"FTLP", *b"RTMT", *b"DRSE", *b"DRSD", *b"TMPT", *b"PENA", *b"SPTP", *b"DTSV", *b"SGSV",
    *b"FLBK", *b"BUTN", *b"RDFL", *b"OVTK", *b"SCAR", *b"COLL",
];

pub fn event(frame: Frame, code: &[u8; 4], lap_seconds: u16) -> BytesMut {
    let mut packet = frame.header(3);
    packet.extend_from_slice(code);
    match code {
        b"SSTA" | b"SEND" | b"DRSE" | b"CHQF" | b"LGOT" | b"RDFL" => {}
        b"FTLP" => {
            packet.put_u8(0);
            packet.put_f32_le(f32::from(lap_seconds));
        }
        b"RTMT" => packet.extend_from_slice(&[21, 8]), // Car 21, mechanical failure
        b"DRSD" => packet.put_u8(1),                   // Safety car deployed
        b"TMPT" | b"DTSV" => packet.put_u8(1),
        b"RCWN" => packet.put_u8(0),
        b"PENA" => {
            packet.extend_from_slice(&[4, 7, 1, 0, 5, completed_laps(frame, lap_seconds) + 1, 0])
        }
        b"SPTP" => {
            packet.put_u8(0);
            packet.put_f32_le(330.0);
            packet.extend_from_slice(&[1, 1, 0]);
            packet.put_f32_le(330.0);
        }
        b"STLG" => packet.put_u8(5),
        b"SGSV" => {
            packet.put_u8(1);
            packet.put_f32_le(10.5); // F1 25 stop-go time
        }
        b"FLBK" => {
            // The target is the current frame: a zero-distance flashback fixture.
            packet.put_u32_le(frame.number);
            packet.put_f32_le(frame.time);
        }
        b"BUTN" => packet.put_u32_le(0x0010_0001), // A/Cross and UDP action 1
        b"OVTK" | b"COLL" => packet.extend_from_slice(&[0, 1]),
        b"SCAR" => packet.extend_from_slice(&[1, 0]), // Full safety car deployed
        _ => panic!("unknown F1 25 event code"),
    }
    assert!(packet.len() <= PACKET_SIZES[3]);
    packet.resize(PACKET_SIZES[3], 0); // Packed event union is always 12 bytes
    packet
}
