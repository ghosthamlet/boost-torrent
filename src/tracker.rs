use std::net::{SocketAddrV4, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use byteorder::{NetworkEndian, NativeEndian, ByteOrder};
use bencode::BencodeValue;
use regex::Regex;
use rand;


#[derive(Debug)]
pub struct TrackerInfo {
    interval: u32,
    seeders: u32,
    leechers: u32,
    tracker_id: Option<String>,
    potential_peers: Vec<PotentialPeer>
}

#[derive(Debug)]
pub struct PotentialPeer {
    addr: SocketAddrV4,
    id: Option<[u8;20]>
}

pub enum TrackerEvent {
    Started,
    Stopped,
    Completed
}

impl TrackerInfo {
    ///send a request to the tracker at the given url, regardless of UDP or HTTP
    pub fn tracker_request<'a>(url: &str, 
                           info_hash: &[u8],
                           peer_id: &[u8], 
                           listen_port: u16,
                           uploaded_bytes: u64, 
                           downloaded_bytes: u64, 
                           bytes_left: u64, 
                           event: TrackerEvent, 
                           tracker_id: Option<&[u8]>) -> Result<Self,&'a str> { 


        //regext to match url, capture proto, domain, port and location
        let re = Regex::new(r"(http|udp)://([a-zA-Z0-9.]+):(\d+)(.*)").unwrap();
        let caps = re.captures(url).ok_or("Could not match the url")?;
        let proto = &caps[1];
        let domain = &caps[2];
        let port = &caps[3].parse::<u16>().map_err(|_|"Could not parse port as integer")?;
        let location = &caps[4];

        //dns resolve, get only ipv4 addrs, get the first one
        let ip = (domain, *port).to_socket_addrs().map_err(|_| "Could not resolve host")?
                    .filter_map(|x| if let SocketAddr::V4(s) = x { Some(s) } else { None })
                    .next()
                    .ok_or("No Addresses Found")?;


        if proto == "udp" {
            udp_tracker_request(ip, info_hash, peer_id, listen_port, uploaded_bytes, downloaded_bytes,
                                bytes_left, event)
        } else if proto == "http" {
            http_tracker_request(ip, info_hash, peer_id, listen_port, uploaded_bytes, downloaded_bytes,
                                bytes_left, event, tracker_id)
        } else {
            Err("URL uses invalid protocol")
        }
    }
}

///Performs a UDP tracker request to the given address
fn udp_tracker_request<'a>(server: SocketAddrV4, 
                        info_hash: &[u8],
                        peer_id: &[u8], 
                        listen_port: u16,
                        uploaded_bytes: u64, 
                        downloaded_bytes: u64, 
                        bytes_left: u64, 
                        event: TrackerEvent) -> Result<TrackerInfo,&'a str> { 

        let mut buf : [u8;512]= [0;512];
        let mut udp_sock = UdpSocket::bind("0.0.0.0:0").map_err(|_| "Could not create a UDP Socket")?;
        let transaction_id = rand::random::<u32>();

        //start udp tracker protocol, send magic number
        NetworkEndian::write_u64(&mut buf[0..8], 0x41727101980);
        //send action (0 = connect)
        NetworkEndian::write_u32(&mut buf[8..12], 0);
        //send transaction_id (random)
        NetworkEndian::write_u32(&mut buf[12..16], transaction_id);
        udp_sock.send_to(&buf[0..16], server).map_err(|_| "Failed to send data over udp 3")?;

        //recieve tracker response
        //recieve and check action
        udp_sock.recv_from(&mut buf[0..16]).map_err(|e| {println!("{:?}",e); "Failed to receive data over udp 4"})?;
        if NetworkEndian::read_u32(&buf[0..4]) != 0 { return Err("Action was supposed to be connect but wasn't")};
        //recieve and check transaction id
        if NetworkEndian::read_u32(&buf[4..8]) != transaction_id { return Err("Transaction ID was Incorrect")};
        //recieve connection id
        let connection_id = NetworkEndian::read_u64(&buf[8..16]);

        //send request info
        NetworkEndian::write_u64(&mut buf[0..8], connection_id);
        //send action (1 = announce)
        NetworkEndian::write_u32(&mut buf[8..12], 1);
        NetworkEndian::write_u32(&mut buf[12..16], transaction_id);
        for i in 0..20 {
            buf[i+16] = info_hash[i];
        }
        for i in 0..20 {
            buf[i+36] = peer_id[i];
        }
        NetworkEndian::write_u64(&mut buf[56..64], downloaded_bytes);
        NetworkEndian::write_u64(&mut buf[64..72], bytes_left);
        NetworkEndian::write_u64(&mut buf[72..80], uploaded_bytes);
        //send which event
        let event = match event {
            TrackerEvent::Completed => 1,
            TrackerEvent::Started => 2,
            TrackerEvent::Stopped => 3
        };
        NetworkEndian::write_u32(&mut buf[80..84], event);
        //ip, key, numwant, all to their defaults
        NetworkEndian::write_u32(&mut buf[84..88], 0);
        NetworkEndian::write_u32(&mut buf[88..92], 0);
        NetworkEndian::write_i32(&mut buf[92..96], -1);
        //write listen port
        NetworkEndian::write_u16(&mut buf[96..98], listen_port);
        udp_sock.send_to(&buf[0..98], server).map_err(|_| "Failed to send data over udp 19")?;

        //recieve tracker info
        //recieve and check action
        udp_sock.recv_from(&mut buf).map_err(|_| "Failed ro recieve over udp 20")?;
        if NetworkEndian::read_u32(&buf[0..4]) != 1 { return Err("Action was supposed to be announce but wasn't")};
        //recieve and check transaction id
        if NetworkEndian::read_u32(&buf[4..8]) != transaction_id { return Err("Transaction id is not correct") };
        let interval = NetworkEndian::read_u32(&buf[8..12]);
        let leechers = NetworkEndian::read_u32(&buf[12..16]);
        let seeders = NetworkEndian::read_u32(&buf[16..20]);

        //while recvfrom doesn't timeout there is still some ips to recv
        let mut idx = 0;
        let mut potential_peers = Vec::new();
        let mut ipbuf = &buf[20+6*idx..20+6*(idx+1)];
        //loop while not at a zeroed out part or not at end
        while ipbuf[0] != 0 && 20+6*idx < 512 {
            let ip = Ipv4Addr::new(ipbuf[0], ipbuf[1], ipbuf[2], ipbuf[3]);
            let port = NetworkEndian::read_u16(&ipbuf[4..6]);
            potential_peers.push(PotentialPeer { addr: SocketAddrV4::new(ip,port), id: None}); 
            idx += 1;
            ipbuf = &buf[20+6*idx..20+6*(idx+1)];
        }
        
        Ok(TrackerInfo { interval, seeders, leechers, tracker_id: None, potential_peers})


}

///performs an HTTP tracker request to the given address
fn http_tracker_request<'a>(server: SocketAddrV4, 
                        info_hash: &[u8],
                        peer_id: &[u8], 
                        listen_port: u16,
                        uploaded_bytes: u64, 
                        downloaded_bytes: u64, 
                        bytes_left: u64, 
                        event: TrackerEvent, 
                        tracker_id: Option<&[u8]>) -> Result<TrackerInfo,&'a str> { 
        Err("Not Done Yet")
}
