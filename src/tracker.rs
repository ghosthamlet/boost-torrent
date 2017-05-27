use std::net::{TcpStream, SocketAddrV4, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use byteorder::{NetworkEndian, NativeEndian, ByteOrder};
use bencode::BencodeValue;
use regex::Regex;
use regex;
use std::str;
use std::io::{Read, Write};
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
    None,
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
                           tracker_id: Option<&str>) -> Result<Self,&'a str> { 


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
            http_tracker_request(ip, domain, location, info_hash, peer_id, listen_port, uploaded_bytes, downloaded_bytes,
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
        let udp_sock = UdpSocket::bind("0.0.0.0:0").map_err(|_| "Could not create a UDP Socket")?;
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
            TrackerEvent::None => 0,
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

///takes a byte slice and returns a byte vec that is the url encoding of the byte slice.
///This means and byte that is not 0-9,A-Z,a-z,.,-,_,~ is %hh where h is the hex value
fn url_encode(data: &[u8]) -> String {
    let mut res = String::new();
    for byte in data {
        match *byte {
            48u8...57u8 | 65u8...90u8 | 97u8...122u8 | 45u8 | 46u8 | 126u8 | 95u8 => res.push(*byte as char),
            b => res+= format!("%{:02X}",b).as_str()
        }
    };
    res
}

///performs an HTTP tracker request to the given address
fn http_tracker_request<'a>(server: SocketAddrV4, 
                            host: &str,
                            location: &str,
                        info_hash: &[u8],
                        peer_id: &[u8], 
                        listen_port: u16,
                        uploaded_bytes: u64, 
                        downloaded_bytes: u64, 
                        bytes_left: u64, 
                        event: TrackerEvent, 
                        tracker_id: Option<&str>) -> Result<TrackerInfo,&'a str> { 
        let encoded_hash = url_encode(info_hash);
        let encoded_id = url_encode(peer_id);

        //why did i roll my own http lol
        //build my request string
        let request_string = format!("GET {}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1&no_peer_id=1", 
                                    location, encoded_hash, encoded_id, listen_port,
                                    uploaded_bytes, downloaded_bytes, bytes_left);
        let event_string = match event {
            TrackerEvent::None => "",
            TrackerEvent::Completed => "event=completed",
            TrackerEvent::Started => "event=started",
            TrackerEvent::Stopped => "event=stopped"
        };
        let tracker_string = tracker_id.unwrap_or("");
        let request_string = format!("{}{}{} HTTP/1.1\r\nUser-Agent: BoostTorrent/0.2\r\nAccept:*/*\r\nHost: {}:{}\r\n\r\n",
                                    request_string, event_string, tracker_string, host, server.port());

        //connect to server and send request
        let mut http_sock = TcpStream::connect(server).map_err(|_| "Could not connect to HTTP server")?;
        let _ = http_sock.write(request_string.as_bytes()).map_err(|_| "could not send HTTP request")?; 
        let mut response = Vec::new();
        let _ = http_sock.read_to_end(&mut response).map_err(|_| "Could not receive HTTP response")?;
        let re = regex::bytes::Regex::new(r"HTTP/.* (\d{3}) OK\r\n((?:.|\s)*)\r\n\r\n((?-u:[\x00-\xff]*))").unwrap();
        let caps = re.captures(response.as_slice()).ok_or("Could not match the response")?;
        let response_code = str::from_utf8(&caps[1]).map_err(|_| "could not parse HTTP error code")?.parse::<u16>().map_err(|_|"Could not read HTTP error code")?;
        let headers = &caps[2];
        let data = &caps[3];

        //Error, not doing redirects yet
        if response_code > 300 {
            Err("HTTP error response")
        } else {
            let tracker_dict = BencodeValue::bdecode(data);
            let mut interval = 0;
            let mut seeders = 0;
            let mut leechers = 0;
            let mut potential_peers = Vec::new();
            let mut tracker_id = None;
            if let Ok(BencodeValue::Dict(tracker_dict)) = tracker_dict {
                //iterate over all availible key/value pairs in the dict
                for (ref key, ref val) in tracker_dict {
                    //if failure, return error
                    if *key == "failure reason".as_bytes() {
                        return Err("The Tracker returned an error")
                    } 
                    //get the interval
                    else if *key == "interval".as_bytes() {
                        if let &BencodeValue::Integer(i) = val {
                            interval = i as u32;
                        } else {
                            return Err("Interval is not an integer")
                        }
                    }
                    //gets the seeders
                    else if *key == "complete".as_bytes() {
                        if let &BencodeValue::Integer(i) = val {
                            seeders = i as u32;
                        } else {
                            return Err("Seeders is not an integer")
                        }
                    }
                    //gets the leechers
                    else if *key == "incomplete".as_bytes() {
                        if let &BencodeValue::Integer(i) = val {
                            leechers = i as u32;
                        } else {
                            return Err("leechers is not an integer")
                        }
                    }
                    //gets the tracker id if there is one
                    else if *key == "tracker id".as_bytes() {
                        if let &BencodeValue::Str(s) = val {
                            tracker_id = Some(String::from(str::from_utf8(s).map_err(|_| "Could not parse tracker id as utf8")?));
                        } else {
                            return Err("Tracker id is not a string")
                        }
                    }
                    //gets the peers info
                    else if *key == "peers".as_bytes() {
                        match val {
                            &BencodeValue::Str(ref peers) => {
                                //peers are in compact mode, ip/port are in 6 byte network byte
                                //order tuples
                                let mut pos = 0;
                                while pos < peers.len() {
                                    let ip = Ipv4Addr::from(NetworkEndian::read_u32(&peers[pos..pos+4]));
                                    let port = NetworkEndian::read_u16(&peers[pos+4..pos+6]);
                                    potential_peers.push(PotentialPeer {addr: SocketAddrV4::new(ip,port), id: None});
                                    pos += 6;
                                }
                            },
                            &BencodeValue::List(ref peers) => {
                                //peers are not compact, they are each a dict
                                for peer in peers {
                                    if let &BencodeValue::Dict(ref peerd) = peer {
                                        let mut id = None;
                                        let mut host = "";
                                        let mut port = 0;
                                        for &(ref pkey, ref pval) in peerd {
                                            //gets the port
                                            if *pkey == "ip".as_bytes() {
                                                if let &BencodeValue::Str(s) = pval {
                                                    host = str::from_utf8(s).map_err(|_| "Could not parse host as utf8 string")?;
                                                } else {
                                                    return Err("Host is not a string")
                                                }
                                            }
                                            //gets the host
                                            else if *pkey == "port".as_bytes() {
                                                if let &BencodeValue::Integer(i) = pval {
                                                    port = i as u16; 
                                                } else {
                                                    return Err("Port is not an integer")
                                                }
                                            }
                                            //gets the id
                                            else if *pkey == "peer id".as_bytes() {
                                                if let &BencodeValue::Str(s) = pval {
                                                    let mut peerid = [0u8;20];
                                                    peerid.copy_from_slice(s);
                                                    id = Some(peerid);
                                                } else {
                                                    return Err("Id is not a string")
                                                }
                                            }
                                        }
                                        //get the sockaddrv4 of the peer from the host and port
                                        let addr = (host,port).to_socket_addrs().map_err(|_| "could not resolve peer's ip")?
                                            .filter_map(|x| if let SocketAddr::V4(s) = x { Some(s) } else { None })
                                            .next().ok_or("Could not find ipv4 address of peer")?;
                                        potential_peers.push(PotentialPeer { addr, id  });
                                    } else {
                                        return Err("Peer is not a dict")
                                    }
                                }
                            },
                            _ => return Err("Peers is not a list or a string")
                        };
                    }
                    
                }
                Ok(TrackerInfo { interval, seeders, leechers, tracker_id, potential_peers })
            } else {
                Err("Response was not a dictionary")
            }
        }
}
