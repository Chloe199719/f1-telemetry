#[derive(Debug, Clone, Copy, PartialEq)]

pub struct PacketHeader {
    // 2  + 1 +
    pub pack_format: u16,       // 2025
    pub game_year: u8,          // Game year  - last 2 digits  expected 25
    pub game_major_version: u8, // Version of the game - expected 1
    pub game_minor_version: u8,
    pub packet_version: u8,
    pub packet_id: PacketId,
    pub session_uid: u64,
    pub session_time: f32,
    pub frame_identifier: u32,
    pub overall_frame_identifier: u32,
    pub player_car_index: u8,
    pub secondary_player_car_index: u8,
}

const HEADER_SIZE: usize = 29; // Size of the header in bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketId {
    Motion = 0,
    Session = 1,
    LapData = 2,
    Event = 3,
    Participants = 4,
    CarSetups = 5,
    CarTelemetry = 6,
    CarStatus = 7,
    FinalClassification = 8,
    LobbyInfo = 9,
    CarDamage = 10,
    SessionHistory = 11,
    TyreSets = 12,
    MotionEx = 13,
    TimeTrial = 14,
    LapPositions = 15,
}

impl PacketHeader {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(anyhow::anyhow!("Packet header is too short"));
        }
        let pack_format = u16::from_le_bytes(bytes[0..2].try_into()?);
        let game_year = bytes[2];
        let game_major_version = bytes[3];
        let game_minor_version = bytes[4];
        let packet_version = bytes[5];
        let packet_id = match bytes[6] {
            0 => PacketId::Motion,
            1 => PacketId::Session,
            2 => PacketId::LapData,
            3 => PacketId::Event,
            4 => PacketId::Participants,
            5 => PacketId::CarSetups,
            6 => PacketId::CarTelemetry,
            7 => PacketId::CarStatus,
            8 => PacketId::FinalClassification,
            9 => PacketId::LobbyInfo,
            10 => PacketId::CarDamage,
            11 => PacketId::SessionHistory,
            12 => PacketId::TyreSets,
            13 => PacketId::MotionEx,
            14 => PacketId::TimeTrial,
            15 => PacketId::LapPositions,
            _ => return Err(anyhow::anyhow!("Unknown packet ID: {}", bytes[6])),
        };
        let session_uid = u64::from_le_bytes(bytes[7..15].try_into()?);
        let session_time = f32::from_le_bytes(bytes[15..19].try_into()?);
        let frame_identifier = u32::from_le_bytes(bytes[19..23].try_into()?);
        let overall_frame_identifier = u32::from_le_bytes(bytes[23..27].try_into()?);
        let player_car_index = bytes[27];
        let secondary_player_car_index = bytes[28];

        Ok(PacketHeader {
            pack_format,
            game_year,
            game_major_version,
            game_minor_version,
            packet_version,
            packet_id,
            session_uid,
            session_time,
            frame_identifier,
            overall_frame_identifier,
            player_car_index,
            secondary_player_car_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PacketHeader, PacketId};

    // Same packed F1 25 header used by the simulator's protocol tests.
    // Bytes 23..27 hold the overall frame ID; player indices follow at 27 and 28.
    const HEADER_BYTES: [u8; 29] = [
        0xe9, 0x07, // Packet format: 2025, little endian
        25, 1, 0, 1, 6, // Game year, major/minor version, packet version, telemetry ID
        8, 7, 6, 5, 4, 3, 2, 1, // Session UID: 0x0102_0304_0506_0708
        0, 0, 0x20, 0x40, // Session time: 2.5f32
        42, 0, 0, 0, // Frame identifier
        90, 0, 0, 0, // Overall frame identifier
        0, 255, // Player car 0, no secondary player
    ];

    #[test]
    fn parses_known_f1_25_header_fields() {
        let header = PacketHeader::from_bytes(&HEADER_BYTES).expect("valid F1 25 header");

        assert_eq!(header.pack_format, 2025);
        assert_eq!(header.game_year, 25);
        assert_eq!(header.game_major_version, 1);
        assert_eq!(header.game_minor_version, 0);
        assert_eq!(header.packet_version, 1);
        assert!(matches!(header.packet_id, PacketId::CarTelemetry));
        assert_eq!(header.session_uid, 0x0102_0304_0506_0708);
        assert_eq!(header.session_time, 2.5);
        assert_eq!(header.frame_identifier, 42);
        assert_eq!(header.player_car_index, 0);
        assert_eq!(header.secondary_player_car_index, 255);
    }

    #[test]
    fn maps_every_packet_id_from_the_specification() {
        let expected_ids = [
            PacketId::Motion,
            PacketId::Session,
            PacketId::LapData,
            PacketId::Event,
            PacketId::Participants,
            PacketId::CarSetups,
            PacketId::CarTelemetry,
            PacketId::CarStatus,
            PacketId::FinalClassification,
            PacketId::LobbyInfo,
            PacketId::CarDamage,
            PacketId::SessionHistory,
            PacketId::TyreSets,
            PacketId::MotionEx,
            PacketId::TimeTrial,
            PacketId::LapPositions,
        ];

        for (id, expected) in expected_ids.iter().enumerate() {
            let mut bytes = HEADER_BYTES;
            bytes[6] = id as u8;

            let header = PacketHeader::from_bytes(&bytes).expect("known packet ID");

            assert_eq!(
                std::mem::discriminant(&header.packet_id),
                std::mem::discriminant(expected),
                "incorrect mapping for packet ID {id}"
            );
        }
    }

    #[test]
    fn rejects_unknown_packet_ids() {
        for id in 16..=u8::MAX {
            let mut bytes = HEADER_BYTES;
            bytes[6] = id;

            assert!(
                PacketHeader::from_bytes(&bytes).is_err(),
                "unknown packet ID {id} must return an error"
            );
        }
    }

    #[test]
    fn rejects_truncated_headers_without_panicking() {
        for length in 0..HEADER_BYTES.len() {
            assert!(
                PacketHeader::from_bytes(&HEADER_BYTES[..length]).is_err(),
                "a {length}-byte header must return an error"
            );
        }
    }

    #[test]
    fn rejects_a_header_missing_its_last_byte() {
        assert!(
            PacketHeader::from_bytes(&HEADER_BYTES[..28]).is_err(),
            "the secondary player index is required even when there is no second player"
        );
    }

    #[test]
    fn parses_header_from_a_full_telemetry_packet() {
        // The body is opaque to the header parser. F1 25 telemetry packets are 1352 bytes.
        let mut packet = vec![0xab; 1352];
        packet[..HEADER_BYTES.len()].copy_from_slice(&HEADER_BYTES);

        let header = PacketHeader::from_bytes(&packet).expect("complete telemetry datagram");

        assert_eq!(header.pack_format, 2025);
        assert!(matches!(header.packet_id, PacketId::CarTelemetry));
        assert_eq!(header.session_uid, 0x0102_0304_0506_0708);
        assert_eq!(header.session_time, 2.5);
        assert_eq!(header.frame_identifier, 42);
    }

    #[test]
    fn reads_split_screen_player_indices_after_the_overall_frame_identifier() {
        let mut bytes = HEADER_BYTES;
        bytes[23..27].copy_from_slice(&0xa1b2_c3d4_u32.to_le_bytes());
        bytes[27] = 7;
        bytes[28] = 21;

        let header = PacketHeader::from_bytes(&bytes).expect("valid split-screen header");

        assert_eq!(header.player_car_index, 7);
        assert_eq!(header.secondary_player_car_index, 21);
    }
}
