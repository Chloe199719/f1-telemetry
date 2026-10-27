use std::{
    collections::BTreeSet,
    net::UdpSocket,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Simulator(Child);

impl Drop for Simulator {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn capture(arguments: &[&str]) -> Vec<Vec<u8>> {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut simulator = Simulator(
        Command::new(env!("CARGO_BIN_EXE_simulator"))
            .args(["--target", &socket.local_addr().unwrap().to_string()])
            .args(arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut packets = Vec::new();
    let mut buffer = [0; 2048];
    while Instant::now() < deadline {
        match socket.recv(&mut buffer) {
            Ok(length) => packets.push(buffer[..length].to_vec()),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if let Some(status) = simulator.0.try_wait().unwrap() {
                    assert!(status.success(), "simulator exited with {status}");
                    return packets;
                }
            }
            Err(error) => panic!("receiving simulator output: {error}"),
        }
    }
    panic!("simulator did not stop before the timeout");
}

#[test]
fn all_packet_ids_and_event_codes_are_delivered_with_complete_headers() {
    let packets = capture(&[
        "--duration",
        "2",
        "--lap-seconds",
        "1",
        "--event-interval-ms",
        "1",
    ]);
    let sizes = [
        1349, 753, 1285, 45, 1284, 1133, 1352, 1239, 1042, 954, 1041, 1460, 231, 273, 101, 1131,
    ];
    let expected_events = [
        *b"SSTA", *b"SEND", *b"FTLP", *b"RTMT", *b"DRSE", *b"DRSD", *b"TMPT", *b"CHQF", *b"RCWN",
        *b"PENA", *b"SPTP", *b"STLG", *b"LGOT", *b"DTSV", *b"SGSV", *b"FLBK", *b"BUTN", *b"RDFL",
        *b"OVTK", *b"SCAR", *b"COLL",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut histories = BTreeSet::new();
    let mut tyre_sets = BTreeSet::new();
    let mut session_uids = BTreeSet::new();
    let mut last_frame = 0;
    let mut last_time = 0.0;
    for packet in &packets {
        assert!(packet.len() >= 29);
        let id = packet[6];
        assert!(id < 16);
        assert_eq!(packet.len(), sizes[usize::from(id)], "packet ID {id}");
        assert_eq!(&packet[..6], &[0xe9, 0x07, 25, 1, 0, 1]);
        assert_eq!(&packet[27..29], &[0, 255]);
        session_uids.insert(packet[7..15].to_vec());
        let time = f32::from_le_bytes(packet[15..19].try_into().unwrap());
        let frame = u32::from_le_bytes(packet[19..23].try_into().unwrap());
        let overall = u32::from_le_bytes(packet[23..27].try_into().unwrap());
        assert!(frame >= last_frame && overall >= frame && time >= last_time);
        (last_frame, last_time) = (frame, time);
        ids.insert(id);
        match id {
            3 => {
                events.insert(<[u8; 4]>::try_from(&packet[29..33]).unwrap());
            }
            11 => {
                histories.insert(packet[29]);
            }
            12 => {
                tyre_sets.insert(packet[29]);
            }
            _ => {}
        }
    }
    assert_eq!(ids, (0..16).collect());
    assert_eq!(events, expected_events);
    assert_eq!(histories, (0..22).collect());
    assert_eq!(tyre_sets, (0..22).collect());
    assert_eq!(session_uids.len(), 1);
    assert_eq!(&packets.last().unwrap()[29..33], b"SEND");
}

#[test]
fn packet_filter_sends_only_selected_types_and_stops_without_events() {
    let packets = capture(&["--packets", "6", "--hz", "20", "--duration", "1"]);
    assert!(!packets.is_empty());
    assert!(
        packets
            .iter()
            .all(|packet| packet.len() == 1352 && packet[6] == 6)
    );
}
