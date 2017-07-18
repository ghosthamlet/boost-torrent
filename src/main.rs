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
mod piece;

use meta::MetaInfo;
use clap::{App,Arg};
use error::BoostError;
use bitvector::BitVector;
use std::sync::{Arc, RwLock};
use peer::{PeerFlags, Peer};
use tracker::PotentialPeer;
use piece::Piece;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::net::{TcpListener, TcpStream, SocketAddr};
use rand::Rng;
use std::{thread, time};


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

    //parse meta file
    let meta_info = meta::MetaInfo::parse_meta(file).unwrap_or_else(|err: BoostError| {
        println!("{}",err);
        std::process::exit(1)
    });

    //set up variables
    let peerid = gen_peer_id();
    let completed = Arc::new(RwLock::new(BitVector::new(meta_info.num_pieces())));
    let active_peers: Arc<RwLock<Vec<Peer>>> = Arc::new(RwLock::new(Vec::new()));
    let potential_peers: Arc<RwLock<Vec<PotentialPeer>>> = Arc::new(RwLock::new(Vec::new()));
    let working_pieces: Arc<RwLock<Vec<Piece>>> = Arc::new(RwLock::new(Vec::new()));
    let total_uploaded = Arc::new(AtomicUsize::new(0));
    let total_downloaded = Arc::new(AtomicUsize::new(0));
    let listener = TcpListener::bind("0.0.0.0:0").expect("Error creating listener socket");
    let listen_port = match listener.local_addr() {
        Ok(SocketAddr::V4(sockv4)) => sockv4.port(),
        Ok(SocketAddr::V6(sockv6)) => sockv6.port(),
        Err(e) => panic!("Error in getting listener port: {}", e)
    };
    let wrap_up = Arc::new(AtomicBool::new(false));
    let mut info_hash = Vec::new();
    info_hash.extend_from_slice(&meta_info.info_hash); 

    //make first call out to tracker
    let mut tracker_info = tracker::TrackerInfo::tracker_request(
        meta_info.announce_url.as_str(),
        &meta_info.info_hash,
        peerid.as_bytes(),
        listen_port,
        total_uploaded.load(Ordering::Relaxed) as u64,
        total_downloaded.load(Ordering::Relaxed) as u64,
        meta_info.file_info.total_bytes(),
        tracker::TrackerEvent::Started,
        None
        ).unwrap_or_else(|err: BoostError| {
        println!("{}",err);
        std::process::exit(2)
    });

    //write the first batch of potential peers
    {
        potential_peers.write()
            .expect("The potential peers lock was poisoned")
            .append(&mut tracker_info.potential_peers)
    };

    //launch threads
    let tracker_thread = start_tracker_request_thread(
        meta_info.announce_url.clone(),
        info_hash.clone(),
        peerid.clone(),
        listen_port,
        tracker_info.interval,
        meta_info.file_info.total_bytes(),
        total_uploaded.clone(),
        total_downloaded.clone(),
        tracker_info.tracker_id.clone(),
        potential_peers.clone(),
        wrap_up.clone()
    );
    //tell infininte looping threads to wrap up so they can be joined
    wrap_up.store(true, Ordering::Relaxed);

    //join all threads
    let _ = tracker_thread.join();

    println!("{:#?}",potential_peers)
}

fn gen_peer_id() -> String {
    let mut rng = rand::thread_rng();
    let chargen = rng.gen_ascii_chars();
    let mut result = String::from("-BO1000-");
    for chr in chargen.take(12) {
        result.push(chr)
    };
    result
}

fn start_tracker_request_thread(url: String,
                                info_hash: Vec<u8>,
                                peer_id: String,
                                listen_port: u16,
                                interval: u32,
                                file_size: u64,
                                uploaded_bytes: Arc<AtomicUsize>,
                                downloaded_bytes: Arc<AtomicUsize>,
                                tracker_id: Option<String>,
                                potential_peers: Arc<RwLock<Vec<PotentialPeer>>>,
                                wrap_up: Arc<AtomicBool>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let second = time::Duration::from_secs(1);
        while !wrap_up.load(Ordering::Relaxed) {
            //every interval, requery tracker and add result to potential peers
            //wait out the interval in 1 second chunks, so can check if need to stop
            for _ in 0 .. interval {
                if wrap_up.load(Ordering::Relaxed) {
                    return
                }
                thread::sleep(second);
            }
            let mut tracker_info = tracker::TrackerInfo::tracker_request(
                url.as_str(),
                info_hash.as_slice(),
                peer_id.as_bytes(),
                listen_port,
                uploaded_bytes.load(Ordering::Relaxed) as u64,
                downloaded_bytes.load(Ordering::Relaxed) as u64,
                file_size,
                tracker::TrackerEvent::None,
                tracker_id.clone()
                ).unwrap_or_else(|err: BoostError| {
                    println!("{}",err);
                    std::process::exit(2)
                });

            potential_peers.write()
                .expect("The potential peers lock was poisoned")
                .append(&mut tracker_info.potential_peers)

        }
    })

}
