#[derive(Debug, Clone, PartialEq)]

pub struct MotionPacket {
    pub world_position_x: f32,     // 4 bytes
    pub world_position_y: f32,     // 4 bytes
    pub world_position_z: f32,     // 4 bytes
    pub world_velocity_x: f32,     // 4 bytes
    pub world_velocity_y: f32,     // 4 bytes
    pub world_velocity_z: f32,     // 4 bytes
    pub world_forward_dir_x: i16,  // 2 bytes
    pub world_forward_dir_y: i16,  // 2 bytes
    pub world_forward_dir_z: i16,  // 2 bytes
    pub world_right_dir_x: i16,    // 2 bytes
    pub world_right_dir_y: i16,    // 2 bytes
    pub world_right_dir_z: i16,    // 2 bytes
    pub g_force_lateral: f32,      // 4 bytes
    pub g_force_longitudinal: f32, // 4 bytes
    pub g_force_vertical: f32,     // 4 bytes
    pub yaw: f32,                  // 4 bytes
    pub pitch: f32,
    pub roll: f32,
}
impl MotionPacket {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Vec<Self>> {
        if bytes.len() < 60 {
            return Err(anyhow::anyhow!("Motion packet is too short"));
        }
        let mut motion_packets = Vec::new();
        for chunk in bytes.chunks_exact(60) {
            let world_position_x = f32::from_le_bytes(chunk[0..4].try_into()?);
            let world_position_y = f32::from_le_bytes(chunk[4..8].try_into()?);
            let world_position_z = f32::from_le_bytes(chunk[8..12].try_into()?);
            let world_velocity_x = f32::from_le_bytes(chunk[12..16].try_into()?);
            let world_velocity_y = f32::from_le_bytes(chunk[16..20].try_into()?);
            let world_velocity_z = f32::from_le_bytes(chunk[20..24].try_into()?);
            let world_forward_dir_x = i16::from_le_bytes(chunk[24..26].try_into()?);
            let world_forward_dir_y = i16::from_le_bytes(chunk[26..28].try_into()?);
            let world_forward_dir_z = i16::from_le_bytes(chunk[28..30].try_into()?);
            let world_right_dir_x = i16::from_le_bytes(chunk[30..32].try_into()?);
            let world_right_dir_y = i16::from_le_bytes(chunk[32..34].try_into()?);
            let world_right_dir_z = i16::from_le_bytes(chunk[34..36].try_into()?);
            let g_force_lateral = f32::from_le_bytes(chunk[36..40].try_into()?);
            let g_force_longitudinal = f32::from_le_bytes(chunk[40..44].try_into()?);
            let g_force_vertical = f32::from_le_bytes(chunk[44..48].try_into()?);
            let yaw = f32::from_le_bytes(chunk[48..52].try_into()?);
            let pitch = f32::from_le_bytes(chunk[52..56].try_into()?);
            let roll = f32::from_le_bytes(chunk[56..60].try_into()?);

            motion_packets.push(MotionPacket {
                world_position_x,
                world_position_y,
                world_position_z,
                world_velocity_x,
                world_velocity_y,
                world_velocity_z,
                world_forward_dir_x,
                world_forward_dir_y,
                world_forward_dir_z,
                world_right_dir_x,
                world_right_dir_y,
                world_right_dir_z,
                g_force_lateral,
                g_force_longitudinal,
                g_force_vertical,
                yaw,
                pitch,
                roll,
            });
        }
        Ok(motion_packets)
    }
}

#[cfg(test)]
mod tests {
    use super::MotionPacket;
    use crate::f1_25::headers::{PacketHeader, PacketId};

    // One CarMotionData record in the PDF's packed little-endian layout.
    // Distinct values make swapped fields, incorrect widths, and signedness visible.
    const MOTION_BYTES: [u8; 60] = [
        0x00, 0x00, 0xf7, 0x42, // world_position_x: 123.5
        0x00, 0x00, 0x35, 0xc2, // world_position_y: -45.25
        0x00, 0xb0, 0x29, 0x44, // world_position_z: 678.75
        0x00, 0x00, 0x48, 0x41, // world_velocity_x: 12.5
        0x00, 0x00, 0x10, 0xc0, // world_velocity_y: -2.25
        0x00, 0x00, 0xa0, 0x42, // world_velocity_z: 80.0
        0x01, 0x80, // world_forward_dir_x: -32767
        0x00, 0xc0, // world_forward_dir_y: -16384
        0xff, 0xff, // world_forward_dir_z: -1
        0x01, 0x00, // world_right_dir_x: 1
        0x00, 0x40, // world_right_dir_y: 16384
        0xff, 0x7f, // world_right_dir_z: 32767
        0x00, 0x00, 0xc0, 0xbf, // g_force_lateral: -1.5
        0x00, 0x00, 0x10, 0x40, // g_force_longitudinal: 2.25
        0x00, 0x00, 0x40, 0xbf, // g_force_vertical: -0.75
        0x00, 0x00, 0x00, 0x3f, // yaw: 0.5
        0x00, 0x00, 0x80, 0xbe, // pitch: -0.25
        0x00, 0x00, 0x00, 0x3e, // roll: 0.125
    ];

    fn assert_sample_fields(motion: &MotionPacket, expected_position_x: f32) {
        assert_eq!(motion.world_position_x, expected_position_x);
        assert_eq!(motion.world_position_y, -45.25);
        assert_eq!(motion.world_position_z, 678.75);
        assert_eq!(motion.world_velocity_x, 12.5);
        assert_eq!(motion.world_velocity_y, -2.25);
        assert_eq!(motion.world_velocity_z, 80.0);
        assert_eq!(motion.world_forward_dir_x, -32767);
        assert_eq!(motion.world_forward_dir_y, -16384);
        assert_eq!(motion.world_forward_dir_z, -1);
        assert_eq!(motion.world_right_dir_x, 1);
        assert_eq!(motion.world_right_dir_y, 16384);
        assert_eq!(motion.world_right_dir_z, 32767);
        assert_eq!(motion.g_force_lateral, -1.5);
        assert_eq!(motion.g_force_longitudinal, 2.25);
        assert_eq!(motion.g_force_vertical, -0.75);
        assert_eq!(motion.yaw, 0.5);
        assert_eq!(motion.pitch, -0.25);
        assert_eq!(motion.roll, 0.125);
    }

    #[test]
    fn parses_all_fields_from_a_known_little_endian_motion_record() {
        let motions = MotionPacket::from_bytes(&MOTION_BYTES).expect("valid 60-byte motion record");

        assert_eq!(motions.len(), 1);
        assert_sample_fields(&motions[0], 123.5);
    }

    #[test]
    fn rejects_every_truncated_motion_record_without_panicking() {
        for length in 0..MOTION_BYTES.len() {
            assert!(
                MotionPacket::from_bytes(&MOTION_BYTES[..length]).is_err(),
                "a {length}-byte motion record must return an error"
            );
        }
    }

    #[test]
    fn preserves_signed_direction_values_including_i16_limits() {
        let mut bytes = MOTION_BYTES;
        let directions = [i16::MIN, -32767, -1, 0, 1, i16::MAX];
        for (index, direction) in directions.iter().enumerate() {
            let offset = 24 + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&direction.to_le_bytes());
        }

        let motions = MotionPacket::from_bytes(&bytes).expect("signed direction components");
        assert_eq!(motions.len(), 1);
        let motion = &motions[0];

        assert_eq!(motion.world_forward_dir_x, i16::MIN);
        assert_eq!(motion.world_forward_dir_y, -32767);
        assert_eq!(motion.world_forward_dir_z, -1);
        assert_eq!(motion.world_right_dir_x, 0);
        assert_eq!(motion.world_right_dir_y, 1);
        assert_eq!(motion.world_right_dir_z, i16::MAX);
        // The neighbouring float fields must remain intact.
        assert_eq!(motion.world_velocity_z, 80.0);
        assert_eq!(motion.g_force_lateral, -1.5);
    }

    #[test]
    fn parses_a_zeroed_car_record() {
        let motions = MotionPacket::from_bytes(&[0; 60]).expect("zeroed car motion data");
        assert_eq!(motions.len(), 1);
        let motion = &motions[0];

        assert_eq!(
            [
                motion.world_position_x,
                motion.world_position_y,
                motion.world_position_z,
                motion.world_velocity_x,
                motion.world_velocity_y,
                motion.world_velocity_z,
                motion.g_force_lateral,
                motion.g_force_longitudinal,
                motion.g_force_vertical,
                motion.yaw,
                motion.pitch,
                motion.roll,
            ],
            [0.0; 12]
        );
        assert_eq!(
            [
                motion.world_forward_dir_x,
                motion.world_forward_dir_y,
                motion.world_forward_dir_z,
                motion.world_right_dir_x,
                motion.world_right_dir_y,
                motion.world_right_dir_z,
            ],
            [0; 6]
        );
    }

    #[test]
    fn parses_all_22_cars_after_the_header_in_a_full_motion_datagram() {
        // Full PacketMotionData: 29-byte header + 22 separate 60-byte car records.
        let header_bytes: [u8; 29] = [
            0xe9, 0x07, 25, 1, 0, 1, 0, // F1 25, packet version 1, Motion ID 0
            8, 7, 6, 5, 4, 3, 2, 1, // Session UID
            0, 0, 0x20, 0x40, // Session time: 2.5f32
            42, 0, 0, 0, // Frame identifier
            90, 0, 0, 0, // Overall frame identifier
            0, 255, // Player car indices
        ];
        let mut datagram = [0; 1349];
        datagram[..29].copy_from_slice(&header_bytes);
        for car in 0..22 {
            let start = 29 + car * 60;
            datagram[start..start + 60].copy_from_slice(&MOTION_BYTES);
            // Give each car a distinct position to catch incorrect record offsets.
            datagram[start..start + 4].copy_from_slice(&(1000.0 + car as f32).to_le_bytes());
        }

        let header = PacketHeader::from_bytes(&datagram).expect("valid motion packet header");
        assert!(matches!(header.packet_id, PacketId::Motion));
        let motions = MotionPacket::from_bytes(&datagram[29..])
            .expect("complete payload containing 22 car records");

        assert_eq!(motions.len(), 22);
        for (car, motion) in motions.iter().enumerate() {
            assert_sample_fields(motion, 1000.0 + car as f32);
        }
    }
}
