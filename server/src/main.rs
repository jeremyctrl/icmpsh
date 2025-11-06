use pnet::{
    packet::{
        Packet,
        icmp::{IcmpTypes, echo_reply, echo_request::EchoRequestPacket},
        ip::IpNextHeaderProtocols,
        util,
    },
    transport::{TransportChannelType, TransportProtocol, icmp_packet_iter, transport_channel},
};

fn main() -> anyhow::Result<()> {
    let (mut tx, mut rx) = transport_channel(
        4096,
        TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Icmp)),
    )?;

    let mut iter = icmp_packet_iter(&mut rx);

    while let Ok((packet, addr)) = iter.next() {
        if packet.get_icmp_type() != IcmpTypes::EchoRequest {
            continue;
        }

        if let Some(echo) = EchoRequestPacket::new(packet.packet()) {
            let mut buf = vec![0u8; packet.packet().len()];
            let mut reply = echo_reply::MutableEchoReplyPacket::new(&mut buf).unwrap();

            reply.set_identifier(echo.get_identifier());
            reply.set_sequence_number(echo.get_sequence_number());
            reply.set_payload(echo.payload());

            let checksum = util::checksum(reply.packet(), 1);
            reply.set_checksum(checksum);

            tx.send_to(reply, addr)?;
        }
    }

    Ok(())
}
