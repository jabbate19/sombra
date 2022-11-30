use default_net::get_default_gateway;
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
use std::net::IpAddr::{V4, V6};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::SystemTime;
// Invoke as echo <interface name>

static cmd_init: &[u8; 22] = b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0HELLO";
static cmd_resp: &[u8; 5] = b"GOODBYE";

//#[derive(Debug)]
pub struct iCMPListener {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    ip_src: Ipv4Addr,
    ip_dst: Ipv4Addr,
    mac_src: MacAddr,
    mac_dst: MacAddr,
}

impl IcmpListener {
    pub fn new(interface_name: String, ip_dst: Ipv4Addr) -> IcmpListener {
        let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;

        // Find the network interface with the provided name
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap();
        let ip_src: Ipv4Addr = match interface.ips.get(0).unwrap().ip() {
            V4(ip) => ip,
            V6(_) => panic!(""),
        };
        let mac_src = interface.mac.unwrap();
        // Create a new channel, dealing with layer 2 packets
        let (tx, rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };
        let mac_dst =
            MacAddr::from_str(&format!("{}", get_default_gateway().unwrap().mac_addr)).unwrap();

        IcmpListener {
            tx,
            rx,
            ip_src,
            ip_dst,
            mac_src,
            mac_dst,
        }
    }
}
impl Read for IcmpListener {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        let start_time = SystemTime::now();
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
                if packet.get_icmp_type().0 == 0 {
                    if &payload[0..5] == cmd_resp {
                        //self.mac_src = eth_packet.get_source();
                        //self.mac_dst = eth_packet.get_destination();
                        //self.ip_src = ip_packet.get_source();
                        //self.ip_dst = ip_packet.get_destination();
                        let cmd = &payload[5..];
                        let len = cmd.len();
                        for i in 0..len {
                            buffer[i] = cmd[i];
                        }
                        return Ok(len);
                    }
                }
            }
            let current_time = SystemTime::now();
            let dur = current_time.duration_since(start_time).unwrap();
            if dur.as_secs_f64() > 10.0 {
                return Ok(0);
            }
        }
    }
}
impl Write for IcmpListener {
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        let mut out = Vec::from(data);
        let mut balls = Vec::from(*cmd_init);
        balls.append(&mut out);

        let mut buffer = [0; 1000];
        let mut new_icmp_packet = MutableIcmpPacket::new(&mut buffer).unwrap();
        new_icmp_packet.set_icmp_type(IcmpType::new(8));
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
        new_ip_packet.set_source(self.ip_src);
        new_ip_packet.set_destination(self.ip_dst);
        new_ip_packet.set_total_length((20 + new_icmp_packet.packet().len()).try_into().unwrap());
        new_ip_packet.set_payload(new_icmp_packet.packet());
        new_ip_packet.set_checksum(checksum(&new_ip_packet.to_immutable()));
        let mut buffer = [0; 1080];
        let mut new_packet = MutableEthernetPacket::new(&mut buffer).unwrap();

        // Switch the source and destination
        new_packet.set_source(self.mac_src);
        new_packet.set_destination(self.mac_dst);
        new_packet.set_ethertype(EtherType::new(0x0800));
        new_packet.set_payload(new_ip_packet.packet());

        let _ = &self.tx.send_to(&new_packet.packet(), None).unwrap();
        return Ok(0);
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        todo!()
    }
}
