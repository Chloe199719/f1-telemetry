pub mod f1_25;

use anyhow::Context;

use crate::f1_25::Packet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:20777")
        .await
        .context("Failed to bind UDP listener on port 20777")?;
    println!(
        "Listening for F1 25 telemetry packets on {}",
        socket.local_addr()?
    );

    let mut buf = [0u8; 10028];
    loop {
        let (len, addr) = socket
            .recv_from(&mut buf)
            .await
            .context("Failed to receive UDP packet")?;

        match Packet::from_bytes(&buf[..len]) {
            Ok(packet) => println!("Package from {addr} ({len} bytes): {:#?}", packet),
            Err(error) => eprintln!("Invalid packet from {addr} ({len} bytes): {error}"),
        }
    }
}
