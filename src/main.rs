extern crate regex;
extern crate byteorder;
extern crate rand;
extern crate sha1;
#[macro_use]
extern crate bitflags;
mod bencode;
mod meta;
mod tracker;
mod peer;
mod bitvector;
mod message;
mod torrentfile;

use meta::MetaInfo;
use meta::FileInfo::Single;

fn main() {
    let file1 = "/home/jake/boostrs/ubuntu-17.04-desktop-amd64.iso.torrent";
    let file2 = "/home/jake/boostrs/research_paper_UDP.torrent";
    if let Ok(MetaInfo { announce_url, piece_len:_, info_hash, piece_hashes: _, file_info: Single {filename:_, filelength}} ) =  meta::MetaInfo::parse_meta(file1) {
        let peerid = "-BO1000-001234567890".as_bytes(); 
        match tracker::TrackerInfo::tracker_request(announce_url.as_str(), &info_hash, peerid, 12345, 0, 0, filelength, tracker::TrackerEvent::Started, None) {
            Ok(t) => println!("{:?}", t),
            Err(e) => println!("Error: {}", e)
        }
    } else {
        println!("error parsing metafile")
    }
}
