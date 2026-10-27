use super::*;

fn frame(time: f32) -> Frame {
    Frame {
        session_uid: 0x0102_0304_0506_0708,
        number: 42,
        overall_number: 90,
        time,
    }
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn f32_at(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

#[test]
fn header_matches_every_field_in_the_pdf_with_distinct_frame_identifiers() {
    assert_eq!(
        &frame(2.5).header(6)[..],
        &[
            0xe9, 0x07, 25, 1, 0, 1, 6, // Format, game and packet versions, ID
            8, 7, 6, 5, 4, 3, 2, 1, // Session UID, little endian
            0, 0, 0x20, 0x40, // Session time: 2.5f32
            42, 0, 0, 0, // Frame identifier
            90, 0, 0, 0, // Overall frame identifier must be written independently
            0, 255, // Player indices
        ]
    );
}

#[test]
fn every_packet_matches_the_pdf_size_at_start_and_after_history_limits() {
    // Literal sizes from the PDF, independent of the encoder's allocation table.
    let sizes = [
        1349, 753, 1285, 45, 1284, 1133, 1352, 1239, 1042, 954, 1041, 1460, 231, 273, 101, 1131,
    ];
    for time in [0.0, 2.5, 510.0, 10_000.0] {
        for lap_seconds in [1, 10, 200, 3600] {
            for id in 0..16 {
                let packet = if id == 3 {
                    event(frame(time), b"SSTA", lap_seconds)
                } else {
                    data(id, frame(time), lap_seconds, 21, 0)
                };
                assert_eq!(packet.len(), sizes[usize::from(id)], "ID {id}, time {time}");
                assert_eq!(packet[6], id);
                assert_eq!(&packet[..6], &[0xe9, 0x07, 25, 1, 0, 1]);
                assert_eq!(u32_at(&packet, 19), 42);
                assert_eq!(u32_at(&packet, 23), 90);
            }
        }
    }
}

#[test]
fn session_includes_full_rules_weekend_and_sector_distances() {
    let packet = data(1, frame(2.5), 10, 0, 0);
    assert_eq!(&packet[29..33], &[0, 35, 24, 100]);
    assert_eq!(u16_at(&packet, 33), 5891);
    assert_eq!(packet[35], 15); // Race
    assert_eq!(packet[36], 7); // Silverstone
    assert_eq!(packet[47], 3); // Marshal zones
    assert_eq!(packet[155], 3); // Forecast samples
    assert_eq!(&packet[156..164], &[15, 0, 0, 35, 2, 24, 2, 0]);
    assert_eq!(u32_at(&packet, 670), 101); // Season link after full forecast array
    assert_eq!(&packet[733..745], &[1, 5, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!((f32_at(&packet, 745) - 5891.0 / 3.0).abs() < 0.001);
    assert!((f32_at(&packet, 749) - 5891.0 * 2.0 / 3.0).abs() < 0.001);
}

#[test]
fn participants_use_32_byte_utf8_names_and_include_four_livery_colours() {
    let packet = data(4, frame(0.0), 10, 0, 0);
    assert_eq!(packet[29], 22);
    for car in 0..22 {
        let start = 30 + car * 57;
        let name = &packet[start + 7..start + 39];
        let end = name.iter().position(|byte| *byte == 0).unwrap();
        assert_eq!(
            std::str::from_utf8(&name[..end]).unwrap(),
            format!("Test Drivér {:02}", car + 1)
        );
        assert_eq!(&name[end..], vec![0; 32 - end]);
        assert_eq!(packet[start + 44], 4);
        assert_eq!(&packet[start + 45..start + 48], &[200, car as u8 * 8, 32]);
        assert_eq!(
            &packet[start + 48..start + 57],
            &[20, 40, 60, 240, 240, 240, 0, 0, 0]
        );
    }
}

#[test]
fn telemetry_values_change_and_wheel_arrays_have_distinct_ordered_values() {
    let packet = data(6, frame(2.5), 10, 0, 0);
    for car in 0..22 {
        let start = 29 + car * 60;
        assert!((90..=330).contains(&u16_at(&packet, start)));
        assert!((0.0..=1.0).contains(&f32_at(&packet, start + 2)));
        assert!((-1.0..=1.0).contains(&f32_at(&packet, start + 6)));
        assert!((0.0..=1.0).contains(&f32_at(&packet, start + 10)));
        assert!((1..=8).contains(&packet[start + 15]));
        assert_eq!(&packet[start + 30..start + 34], &[85, 86, 87, 88]);
        assert_eq!(&packet[start + 34..start + 38], &[90, 91, 92, 93]);
        assert_eq!(u16_at(&packet, start + 38), 105);
        assert_eq!(&packet[start + 56..start + 60], &[0, 0, 1, 1]);
    }
    assert_eq!(&packet[1349..], &[255, 255, 0]);
    assert_ne!(&packet[29..1349], &data(6, frame(5.0), 10, 0, 0)[29..1349]);
    assert_eq!(packet, data(6, frame(2.5), 10, 0, 0));
}

#[test]
fn status_has_c6_and_damage_has_blisters_before_wing_fields() {
    let status = data(7, frame(15.0), 10, 0, 0);
    assert_eq!(&status[29 + 25..29 + 29], &[22, 16, 1, 1]);
    let damage = data(10, frame(15.0), 10, 0, 0);
    for car in 0..22 {
        let start = 29 + car * 46;
        assert_eq!(&damage[start + 24..start + 28], &[9, 10, 11, 12]);
        assert_eq!(&damage[start + 28..start + 34], &[1, 2, 3, 4, 5, 6]);
        assert_eq!(damage[start + 36], car as u8);
        assert_eq!(&damage[start + 44..start + 46], &[0, 0]);
    }
}

#[test]
fn classification_includes_result_reason_and_f64_race_time() {
    let packet = data(8, frame(25.0), 10, 0, 0);
    assert_eq!(packet[29], 22);
    let second_car = 30 + 46;
    assert_eq!(
        &packet[second_car..second_car + 7],
        &[2, 2, 2, 18, 0, 3, 10]
    );
    assert_eq!(u32_at(&packet, second_car + 7), 10_000);
    assert_eq!(
        f64::from_le_bytes(packet[second_car + 11..second_car + 19].try_into().unwrap()),
        25.25
    );
    assert_eq!(packet[second_car + 21], 1);
    assert_eq!(packet[second_car + 22], 22);
    assert_eq!(packet[second_car + 30], 16);
    assert_eq!(packet[second_car + 38], 2);
}

#[test]
fn history_encodes_minute_parts_validity_flags_and_current_stint() {
    let packet = data(11, frame(210.0), 200, 21, 0);
    assert_eq!(&packet[29..36], &[21, 2, 1, 1, 1, 1, 1]);
    assert_eq!(u32_at(&packet, 36), 200_000);
    assert_eq!(u16_at(&packet, 40), 6666);
    assert_eq!(packet[42], 1); // Sector 1 whole minute part
    assert_eq!(packet[49], 0x0f); // First complete lap's validity flags
    assert_eq!(u32_at(&packet, 50), 10_000); // Current partial lap
    assert_eq!(packet[63], 0); // Partial lap flags
    assert_eq!(&packet[1436..1439], &[255, 22, 16]);
    assert_eq!(&packet[1439..], &[0; 21]);
    assert_eq!(data(11, frame(10_000.0), 10, 0, 0)[30], 100);
}

#[test]
fn tyre_sets_include_signed_deltas_wet_compounds_and_fitted_index() {
    let packet = data(12, frame(10.0), 10, 21, 0);
    assert_eq!(packet[29], 21);
    assert_eq!(&packet[30..40], &[22, 16, 1, 1, 15, 20, 18, 0, 0, 1]);
    assert_eq!(i16::from_le_bytes(packet[47..49].try_into().unwrap()), -125);
    assert_eq!(&packet[160..162], &[7, 7]);
    assert_eq!(&packet[200..202], &[8, 8]);
    assert_eq!(packet[230], 0);
}

#[test]
fn motion_ex_includes_chassis_pitch_camber_and_camber_gain_at_v3_offsets() {
    let packet = data(13, frame(2.5), 10, 0, 0);
    assert_eq!(f32_at(&packet, 237), -0.03);
    for (wheel, expected) in [-0.04, -0.05, -0.06, -0.07].into_iter().enumerate() {
        assert_eq!(f32_at(&packet, 241 + wheel * 4), expected);
    }
    for (wheel, expected) in [0.001, 0.002, 0.003, 0.004].into_iter().enumerate() {
        assert_eq!(f32_at(&packet, 257 + wheel * 4), expected);
    }
}

#[test]
fn time_trial_has_three_24_byte_datasets_with_sector_totals() {
    let packet = data(14, frame(0.0), 10, 0, 0);
    for car in 0..3 {
        let start = 29 + car * 24;
        assert_eq!(packet[start], car as u8);
        assert_eq!(
            u32_at(&packet, start + 2),
            u32_at(&packet, start + 6) + u32_at(&packet, start + 10) + u32_at(&packet, start + 14)
        );
        assert_eq!(&packet[start + 18..start + 24], &[0, 0, 0, 1, 1, 1]);
    }
}

#[test]
fn lap_positions_split_at_50_laps_and_clear_unused_rows() {
    let first = data(15, frame(510.0), 10, 0, 0);
    let second = data(15, frame(510.0), 10, 0, 50);
    assert_eq!(&first[29..31], &[50, 0]);
    assert_eq!(&second[29..31], &[2, 50]);
    assert_eq!(&second[31..53], &(1..=22).collect::<Vec<_>>());
    assert_eq!(&second[75..], &[0; 1056]);
}

#[test]
fn all_21_event_codes_have_correct_detail_sizes_and_union_padding() {
    let detail_sizes = [
        0, 0, 5, 2, 0, 1, 1, 0, 1, 7, 12, 1, 0, 1, 5, 8, 4, 0, 2, 2, 2,
    ];
    assert_eq!(EVENT_CODES.len(), 21);
    for (code, size) in EVENT_CODES.iter().zip(detail_sizes) {
        let packet = event(frame(2.5), code, 10);
        assert_eq!(packet.len(), 45);
        assert_eq!(packet[6], 3);
        assert_eq!(&packet[29..33], code);
        assert!(
            packet[33 + size..].iter().all(|byte| *byte == 0),
            "padding for {code:?}"
        );
    }
    assert_eq!(&event(frame(2.5), b"RTMT", 10)[33..35], &[21, 8]);
    assert_eq!(event(frame(2.5), b"DRSD", 10)[33], 1);
    assert_eq!(f32_at(&event(frame(2.5), b"SGSV", 10), 34), 10.5);
    let trap = event(frame(2.5), b"SPTP", 10);
    assert_eq!(f32_at(&trap, 34), 330.0);
    assert_eq!(&trap[38..41], &[1, 1, 0]);
    assert_eq!(f32_at(&trap, 41), 330.0);
    let flashback = event(frame(2.5), b"FLBK", 10);
    assert_eq!(u32_at(&flashback, 33), 42);
    assert_eq!(f32_at(&flashback, 37), 2.5);
    assert_eq!(u32_at(&event(frame(2.5), b"BUTN", 10), 33), 0x0010_0001);
}
