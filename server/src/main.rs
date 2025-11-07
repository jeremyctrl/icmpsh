use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::thread;

use pnet::{
    packet::{
        Packet,
        icmp::{IcmpPacket, IcmpTypes, echo_reply, echo_request::EchoRequestPacket},
        ip::IpNextHeaderProtocols,
        util,
    },
    transport::{
        TransportChannelType, TransportProtocol, TransportSender, icmp_packet_iter,
        transport_channel,
    },
};

use crate::app::App;

mod app;

const SIGNATURE: [u8; 24] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, b'i', b'c', b'm', b'p', b's', b'h', 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = transport_channel(
        4096,
        TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Icmp)),
    )?;

    let tx = Arc::new(Mutex::new(tx));
    let tx_clone = tx.clone();

    thread::spawn(move || {
        let mut iter = icmp_packet_iter(&mut rx);

        while let Ok((packet, addr)) = iter.next() {
            if packet.get_icmp_type() != IcmpTypes::EchoRequest {
                continue;
            }

            if let Err(e) = process_packet(addr, packet, &tx_clone) {
                eprintln!("error processing packet: {:?}", e);
            }
        }
    });

    let mut app = App::new();
    
    app.run()
}

fn process_packet(
    addr: IpAddr,
    icmp: IcmpPacket,
    tx: &Arc<Mutex<TransportSender>>,
) -> anyhow::Result<()> {
    if let Some(echo) = EchoRequestPacket::new(icmp.packet()) {
        if !echo.payload().starts_with(&SIGNATURE) {
            let mut buf = vec![0u8; icmp.packet().len()];
            let mut reply = echo_reply::MutableEchoReplyPacket::new(&mut buf).unwrap();

            reply.set_identifier(echo.get_identifier());
            reply.set_sequence_number(echo.get_sequence_number());
            reply.set_payload(echo.payload());

            let checksum = util::checksum(reply.packet(), 1);
            reply.set_checksum(checksum);

            tx.lock().unwrap().send_to(reply, addr)?;

            return Ok(());
        }

        println!("connection from {:?}", addr);
    }

    Ok(())
}
