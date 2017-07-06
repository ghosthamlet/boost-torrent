extern crate regex;
extern crate byteorder;
extern crate rand;
extern crate sha1;
#[macro_use]
extern crate bitflags;
extern crate clap;
mod bencode;
mod meta;
mod tracker;
mod peer;
mod bitvector;
mod message;
mod torrentfile;
mod error;

use meta::MetaInfo;
use meta::FileInfo::Single;
use clap::{App,Arg};

fn main() {
    //get command line arguments
    let args = App::new(env!("CARGO_PKG_NAME"))
                        .version(env!("CARGO_PKG_VERSION"))
                        .author(env!("CARGO_PKG_AUTHORS"))
                        .about("A torrent client written in rust")
                        .arg(
                            Arg::with_name("meta")
                                .required(true)
                                .short("m")
                                .long("meta")
                                .takes_value(true)
                                .help("The torrent's metafile")
                        ).get_matches();

    let file = args.value_of("meta").unwrap();
    match  meta::MetaInfo::parse_meta(file) {
        Ok(MetaInfo { announce_url, piece_len:_, info_hash, piece_hashes: _, file_info: Single {filename:_, filelength}} ) => {
            let peerid = "-BO1000-001234567890".as_bytes(); 
            match tracker::TrackerInfo::tracker_request(announce_url.as_str(), &info_hash, peerid, 12345, 0, 0, filelength, tracker::TrackerEvent::Started, None) {
                Ok(t) => println!("{:?}", t),
                Err(e) => println!("Error: {}", e)
            }
        },
        Err(e) => println!("{}",e),
        _ => println!("?")

    }
}
