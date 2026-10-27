pub mod headers;
pub mod motion;
#[derive(Debug, Clone, PartialEq)]

pub struct Packet {
    pub header: headers::PacketHeader,
    pub data: Option<PacketType>,
}
impl Packet {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let header = headers::PacketHeader::from_bytes(bytes)?;
        let data = match header.packet_id {
            headers::PacketId::Motion => Some(PacketType::Motion(
                motion::MotionPacket::from_bytes(&bytes[29..])?,
            )),
            _ => None,
        };
        Ok(Packet { header, data })
    }
}
#[derive(Debug, Clone, PartialEq)]

pub enum PacketType {
    Motion(Vec<motion::MotionPacket>),
    Session,
    LapData,
    Event,
    Participants,
    CarSetups,
    CarTelemetry,
    CarStatus,
    FinalClassification,
    LobbyInfo,
    CarDamage,
    SessionHistory,
    TyreSets,
    MotionEx,
    TimeTrial,
    LapPositions,
}
