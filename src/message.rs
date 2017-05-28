use std::net::TcpStream;
use byteorder::{NetworkEndian, ByteOrder};
use std::io::{Read, Write};
use bitvector::BitVector;

///An enum that represents the possible messages BitTorrent can send
pub enum BitTorrentMessage {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(BitVector),
    Request { piece_index: u32, begin: u32, length: u32 },
    Piece { piece_index: u32, begin: u32, block: Vec<u8>},
    Cancel { piece_index: u32, begin: u32, length: u32 },
}

impl BitTorrentMessage {
    ///Encodes self as the on-the-wire form and sends the result to the given dest
    pub fn send(&self, dest: &mut TcpStream) -> Result<(),String> {
        let mut msg = Vec::new(); //holds the data
        let mut u32bytebuf = [0u8; 4]; //used to convert u32 to network byte order byte arrays
        let mut send_buf = Vec::new(); //will be concat of msglen and msg
        match self {
            &BitTorrentMessage::KeepAlive => (), //just send 0 len msg
            //for these messages, just send 1 byte id
            &BitTorrentMessage::Choke => msg.push(0),
            &BitTorrentMessage::Unchoke => msg.push(1),
            &BitTorrentMessage::Interested => msg.push(2),
            &BitTorrentMessage::NotInterested => msg.push(3),
            //send the msg id and the piece you have
            &BitTorrentMessage::Have(piece) => {
                msg.push(4);
                NetworkEndian::write_u32(&mut u32bytebuf, piece);
                msg.extend_from_slice(&u32bytebuf);
            },
            //send msg id and the bytes of the bitvector
            &BitTorrentMessage::Bitfield(ref bitvec) => {
               msg.push(5);
               msg.extend_from_slice(bitvec.as_bytes());
            },
            //send the id and the index, begin and length
            &BitTorrentMessage::Request { piece_index, begin, length } => {
                msg.push(6);
                NetworkEndian::write_u32(&mut u32bytebuf, piece_index);
                msg.extend_from_slice(&u32bytebuf);
                NetworkEndian::write_u32(&mut u32bytebuf, begin);
                msg.extend_from_slice(&u32bytebuf);
                NetworkEndian::write_u32(&mut u32bytebuf, length);
                msg.extend_from_slice(&u32bytebuf);
            },
            //send id, index, begin and block data bytes
            &BitTorrentMessage::Piece { piece_index, begin, ref block} => {
                msg.push(7);
                NetworkEndian::write_u32(&mut u32bytebuf, piece_index);
                msg.extend_from_slice(&u32bytebuf);
                NetworkEndian::write_u32(&mut u32bytebuf, begin);
                msg.extend_from_slice(&u32bytebuf);
                msg.extend_from_slice(block.as_slice());
            },
            //same as request but with id 8 instead of id 6
            &BitTorrentMessage::Cancel { piece_index, begin, length } => {
                msg.push(8);
                NetworkEndian::write_u32(&mut u32bytebuf, piece_index);
                msg.extend_from_slice(&u32bytebuf);
                NetworkEndian::write_u32(&mut u32bytebuf, begin);
                msg.extend_from_slice(&u32bytebuf);
                NetworkEndian::write_u32(&mut u32bytebuf, length);
                msg.extend_from_slice(&u32bytebuf);

            }
        }
        NetworkEndian::write_u32(&mut u32bytebuf, msg.len() as u32); 
        send_buf.extend_from_slice(&u32bytebuf);
        send_buf.append(&mut msg);
        dest.write(send_buf.as_slice()).map(|_| ()).map_err(|_| String::from("Could not send data to peer"))
    }

    ///Recieves a message from the src and decodes it to self 
    pub fn recv(src: &mut TcpStream) -> Result<Self,String> {
        let mut u32bytebuf = [0u8;4];
        src.read(&mut u32bytebuf).map_err(|_| "Could not receive data from peer 1")?;
        let msglen = NetworkEndian::read_u32(&u32bytebuf);

        if msglen == 0 {
            Ok(BitTorrentMessage::KeepAlive)
        } else {
            let mut data = Vec::with_capacity(msglen as usize);
            src.read(data.as_mut_slice()).map_err(|_| "Could not receive data from peer 2")?;
            let msgid = data[0]; //get message id
            match msgid {
                1 => Ok(BitTorrentMessage::Choke),
                2 => Ok(BitTorrentMessage::Unchoke),
                3 => Ok(BitTorrentMessage::Interested),
                4 => Ok(BitTorrentMessage::NotInterested),
                5 => {
                    Ok(BitTorrentMessage::Have(NetworkEndian::read_u32(&data[1..5])))
                },
                6 => {
                    let piece_index = NetworkEndian::read_u32(&data[1..5]);
                    let begin = NetworkEndian::read_u32(&data[5..9]);
                    let length = NetworkEndian::read_u32(&data[9..13]);
                    Ok(BitTorrentMessage::Request {piece_index, begin, length})
                },
                7 => {
                    let piece_index = NetworkEndian::read_u32(&data[1..5]);
                    let begin = NetworkEndian::read_u32(&data[5..9]);
                    let mut block = Vec::new();
                    block.extend_from_slice(&data[9..data.len()]);
                    Ok(BitTorrentMessage::Piece {piece_index, begin, block})
                },
                8 => {
                    let piece_index = NetworkEndian::read_u32(&data[1..5]);
                    let begin = NetworkEndian::read_u32(&data[5..9]);
                    let length = NetworkEndian::read_u32(&data[9..13]);
                    Ok(BitTorrentMessage::Cancel {piece_index, begin, length})
                }
                i => Err(format!("Message Id '{}' is not recognized", i))
            }
        }

    }
}
