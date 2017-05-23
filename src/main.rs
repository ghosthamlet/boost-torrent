mod bencode;
mod meta;

use bencode::BencodeValue;
use std::str;

fn main() {
    let file = "/home/jake/boostrs/ubuntu-17.04-desktop-amd64.iso.torrent";
    match meta::MetaInfo::parse_meta(file) {
        Ok(meta) => println!("{:?}",meta),
        Err(e) => println!("parse error: {}", e)
    }
}

fn print_bval(data: &BencodeValue) {
    match data {
        &BencodeValue::Integer(i) => println!("int: {}", i),
        &BencodeValue::Str(s) => {
            if let Ok(s) = str::from_utf8(s) {
                println!("string: {}", s);
            } else {
                println!("string: Binary data");
            }
        },
        &BencodeValue::List(ref l) => {
            println!("list:");
            for val in l {
                print_bval(&val);
            }
            println!("End list");
        },
        &BencodeValue::Dict(ref d) => {
            println!("dict:");
            for &(k,ref val) in d {
                println!("key: {}", str::from_utf8(k).unwrap());
                print_bval(&val);
            }
            println!("End dict");
        }
    }
}
