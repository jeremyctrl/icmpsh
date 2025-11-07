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

use crate::app::{App, Recipient};

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
    let tx_clone: Arc<Mutex<TransportSender>> = tx.clone();

    let app = App::new();
    let recipients = app.recipients.clone();

    thread::spawn(move || {
        let mut iter = icmp_packet_iter(&mut rx);

        while let Ok((packet, addr)) = iter.next() {
            if packet.get_icmp_type() != IcmpTypes::EchoRequest {
                continue;
            }

            if let Err(e) = process_packet(addr, packet, &tx_clone, &recipients) {
                eprintln!("error processing packet: {:?}", e);
            }
        }
    });

    app.run()
}

fn process_packet(
    addr: IpAddr,
    icmp: IcmpPacket,
    tx: &Arc<Mutex<TransportSender>>,
    recipients: &Arc<Mutex<Vec<Recipient>>>,
) -> anyhow::Result<()> {
    if let Some(echo) = EchoRequestPacket::new(icmp.packet()) {
        let payload = echo.payload();

        if !payload.starts_with(&SIGNATURE) {
            let mut buf = vec![0u8; icmp.packet().len()];
            let mut reply = echo_reply::MutableEchoReplyPacket::new(&mut buf).unwrap();

            reply.set_identifier(echo.get_identifier());
            reply.set_sequence_number(echo.get_sequence_number());
            reply.set_payload(payload);

            let checksum = util::checksum(reply.packet(), 1);
            reply.set_checksum(checksum);

            tx.lock().unwrap().send_to(reply, addr)?;

            return Ok(());
        }

        let label = addr.to_string();
        let mut recipients = recipients.lock().unwrap();

        let idx = recipients.iter().position(|r| r.label == label);
        let idx = match idx {
            Some(i) => i,
            None => {
                recipients.push(Recipient::new(&label));
                recipients.len() - 1
            }
        };
        let rec = &mut recipients[idx];

        if payload.len() != SIGNATURE.len() {
            let data = &payload[SIGNATURE.len()..];
            if let Ok(msg) = String::from_utf8(data.to_vec()) {
                rec.add_message(&msg);
                rec.queued.clear();
                rec.blocked = false;
            }
        }

        let mut data = SIGNATURE.to_vec();
        data.extend_from_slice(rec.queued.as_bytes());

        let mut buf = vec![0u8; 8 + data.len()];
        let mut reply = echo_reply::MutableEchoReplyPacket::new(&mut buf).unwrap();

        reply.set_identifier(echo.get_identifier());
        reply.set_sequence_number(echo.get_sequence_number());
        reply.set_payload(&data);

        let checksum = util::checksum(reply.packet(), 1);
        reply.set_checksum(checksum);

        tx.lock().unwrap().send_to(reply, addr)?;
    }

    Ok(())
}
