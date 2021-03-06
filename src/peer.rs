use bitvector::BitVector;
use std::net::TcpStream;
use byteorder::{NetworkEndian, ByteOrder};
use std::io::{Read, Write};
use std::str;
use error::{BoostError, BoostResult};
use message::BitTorrentMessage;

bitflags! {
    pub struct PeerFlags: u32 {
        const CHOKED            = 0b00000001;
        const CHOKING           = 0b00000010;
        const INTERESTED_IN_ME  = 0b00000100;
        const INTERESTED_IN_THEM= 0b00001000;
        const INCOMING          = 0b00010000;
    }
}


///A struct that represents a connected peer
pub struct Peer {
    pub id: [u8; 20],
    pub socket: TcpStream,
    bytes_sent: u32,
    bytes_received: u32,
    bit_vector: BitVector,
    flags: PeerFlags,
    pending_requests: u32
}

impl Peer {
    ///Takes a freshly created tcp socket, as well as this client's id, the info hash, the
    ///number of pieces and whether this connection is incoming or outgoing,
    ///and performs the handshake, and starts the connection!
    pub fn start_session(mut sock: TcpStream, my_id: &[u8], info_hash: &[u8], num_pieces: usize, incoming: bool) -> BoostResult<Self> {
        let mut handshake_buf = Vec::new();

        //proto string len
        handshake_buf.push(19);
        //proto string
        handshake_buf.extend_from_slice("BitTorrent protocol".as_bytes());
        //reserved bytes (all zeros)
        handshake_buf.extend_from_slice(&[0;8]);
        //info hash
        if info_hash.len() == 20 {
            handshake_buf.extend_from_slice(info_hash);
        } else {
            return Err(BoostError::BitTorrentProtocolErr(format!("Info hash has incorrect length: {} != 20", info_hash.len())))
        }
        //my id
        if my_id.len() == 20 {
            handshake_buf.extend_from_slice(my_id);
        } else {
            return Err(BoostError::BitTorrentProtocolErr(format!("Id has incorrect length: {} != 20", my_id.len())))
        }

        //send handshake
        let _ = sock.write(handshake_buf.as_slice()).map_err(|_| BoostError::BitTorrentTCPSendErr)?;

        //recieve handshake
        let _ = sock.read(handshake_buf.as_mut_slice()).map_err(|_| BoostError::BitTorrentTCPRecvErr)?;

        //check protocol string
        if handshake_buf[0] == 19 {
            if "BitTorrent protocol" == str::from_utf8(&handshake_buf[1..20]).map_err(|_| BoostError::BitTorrentProtocolErr(String::from("Protocol string was not a string")))? {
                //check info hash
                if info_hash == &handshake_buf[28..48] {
                    let mut id = [0u8; 20];
                    let mut flags = PeerFlags::empty();
                    flags.set(INCOMING, incoming);
                    //get peer id
                    for idx in 0..20 {
                       id[idx] = handshake_buf[idx+48];
                    }
                    Ok(Peer {id, socket: sock, bytes_sent: 0, bytes_received: 0, bit_vector: BitVector::new(num_pieces), flags: flags, pending_requests: 0})
                } else {
                    Err(BoostError::BitTorrentProtocolErr(String::from("Info hash was not
                    correct")))
                }
            } else {
                Err(BoostError::BitTorrentProtocolErr(String::from("Proto string was not
                                                                   correct")))
            }
        } else {
            Err(BoostError::BitTorrentProtocolErr(format!("Recieved handshake protocol string length was {} != 19", handshake_buf[0])))
        }
    }


    ///blocks listening for a message. If there is some kind of IO error,
    ///this peer should be removed from the active peers
    pub fn recv_message(&mut self) -> BoostResult<BitTorrentMessage> {
        BitTorrentMessage::recv(&mut self.socket)
    }

    ///Sends the given message to this peer.  If there is some kind of IO error,
    ///this peer should be removed from the active peers
    pub fn send_message(&mut self, message: BitTorrentMessage) -> BoostResult<()> {
        message.send(&mut self.socket)
    }

    ///tells whether this peer was created by connecting to it (outgoing)
    ///or by it connecting to us (incoming)
    pub fn is_incoming(&self) -> bool {
        return !(self.flags & INCOMING).is_empty()
    }

}
