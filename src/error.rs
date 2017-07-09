use std::result;
use message::BitTorrentMessage;
use std::fmt;

pub enum BoostError {
    FileOpenErr(String),
    FileReadErr(String),
    FileWriteErr(String),
    BencodeDecodingErr,
    BencodeEncodingErr,
    BencodeValueErr(String),
    TrackerURLParseErr,
    TrackerHostResolveErr,
    TrackerUDPSendErr,
    TrackerUDPRecvErr,
    TrackerUDPProtocolErr,
    TrackerHTTPConnectErr,
    TrackerHTTPSendErr,
    TrackerHTTPRecvErr,
    TrackerHTTPProtocolErr,
    TorrentFileMetaErr,
    TorrentFileAllocationErr,
    BitTorrentProtocolErr(String),
    BitTorrentTCPSendErr,
    BitTorrentTCPRecvErr,
    UnexpectedMessageType(BitTorrentMessage)
}

impl fmt::Display for BoostError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BoostError::FileOpenErr(ref file) => write!(f, "Error opening {}", file),
            BoostError::FileReadErr(ref file) => write!(f, "Error reading from {}", file),
            BoostError::FileWriteErr(ref file) => write!(f, "Error writing to {}", file),
            BoostError::BencodeDecodingErr => write!(f, "Error decoding a bencoded string"),
            BoostError::BencodeEncodingErr => write!(f, "Error encoding to a bencoded string"),
            BoostError::BencodeValueErr(ref msg) => write!(f, "The bencoded value was structured differently than expected: {}", msg),
            BoostError::TrackerURLParseErr => write!(f, "The tracker URL could not be parsed"),
            BoostError::TrackerHostResolveErr => write!(f, "Could not reslove the host to an IP"),
            BoostError::TrackerUDPSendErr => write!(f, "Error sending data to the tracker over UDP"),
            BoostError::TrackerUDPRecvErr => write!(f, "Error receiving data from the tracker over UDP"),
            BoostError::TrackerUDPProtocolErr => write!(f, "Error communicating with the tracker"),
            BoostError::TrackerHTTPConnectErr => write!(f, "Could not connect to the HTTP tracker"),
            BoostError::TrackerHTTPSendErr => write!(f, "Error sending data to the HTTP tracker"),
            BoostError::TrackerHTTPRecvErr => write!(f, "Error receiving data from the HTTP tracker"),
            BoostError::TrackerHTTPProtocolErr => write!(f, "Error communicating with the HTTP tracker"),
            BoostError::TorrentFileMetaErr => write!(f, "Could not understand the meta info"),
            BoostError::TorrentFileAllocationErr => write!(f, "Could not allocate disk space for torrent file"),
            BoostError::BitTorrentProtocolErr(ref msg) => write!(f, "Error communicating with a peer: {}", msg),
            BoostError::BitTorrentTCPSendErr => write!(f, "Error sending data to peer over TCP"),
            BoostError::BitTorrentTCPRecvErr => write!(f, "Error recieving data from peer over TCP"),
            BoostError::UnexpectedMessageType(ref msg) => write!(f, "Got an unexpected message type: {}", msg)
        }
    }
}

pub type BoostResult<T> = result::Result<T, BoostError>;
