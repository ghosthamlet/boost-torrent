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
use error::BoostError;

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

    //set up variables
    let peerid = "-BO1000-001234567890".as_bytes(); 

    //parse meta file
    let meta_info = meta::MetaInfo::parse_meta(file).unwrap_or_else(|err: BoostError| {
        println!("{}",err);
        std::process::exit(1)
    });

    //call out to tracker
    let tracker_info = tracker::TrackerInfo::tracker_request(
        meta_info.announce_url.as_str(), 
        &meta_info.info_hash, 
        peerid, 
        12345, 
        0, 
        0, 
        meta_info.file_info.total_bytes(),
        tracker::TrackerEvent::Started, 
        None
    ).unwrap_or_else(|err: BoostError| {
        println!("{}",err);
        std::process::exit(2)
    });

    println!("{:#?}",tracker_info)
}

