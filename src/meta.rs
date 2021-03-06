use bencode::BencodeValue;
use std::fs::File;
use std::io::prelude::*;
use sha1::Sha1;
use std::str;
use error::{BoostError, BoostResult};

#[derive(Debug)]
pub struct MetaInfo {
    pub announce_url : String,
    pub piece_len : u64,
    pub info_hash : [u8; 20],
    pub piece_hashes : Vec<[u8; 20]>,
    pub file_info : FileInfo
}

#[derive(Debug)]
pub enum FileInfo {
    Single { filename : String, filelength : u64 },
    Multi { rootdir : String, files : Vec<FileInfo> }
}

impl MetaInfo {
        ///Parses the given metafile and returns a filled out MetaInfo struct
        pub fn parse_meta(torrent_file : &str) -> BoostResult<Self> {
            //read from file
            let mut file = File::open(torrent_file).map_err(|_|
                                                            BoostError::FileOpenErr(String::from(torrent_file)))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).map_err(|_|
                                               BoostError::FileReadErr(String::from(torrent_file)))?;
            //get the bencoded info, ensure its a dictionary
            if let Ok(dict) = BencodeValue::bdecode(buf.as_slice()) {
                let announce_url = parse_announce(&dict)?;
                let (piece_len, piece_hashes) = parse_pieces(&dict)?;
                let file_info = parse_fileinfo(&dict)?;
                let info_hash = make_info_hash(&dict)?;
                Ok(MetaInfo { announce_url, piece_len, info_hash, piece_hashes, file_info })
            } else {
                Err(BoostError::BencodeValueErr(String::from("Metafile bencode Toplevel not a dict")))
            }
        }

        ///Gets the number of pieces this torrent has
        pub fn num_pieces(&self) -> usize {
            let bytes = self.file_info.total_bytes();
            //if piece length does not evenly divide bytes, there will be an extra piece
            let extra = if bytes % self.piece_len == 0 { 0 } else { 1 };
            (bytes / self.piece_len + extra) as usize
        }

}

impl FileInfo {
    ///Gets the total number of bytes that this torrent will require on disk
    pub fn total_bytes(&self) -> u64 {
        match *self {
            FileInfo::Single { filelength, .. } => filelength,
            FileInfo::Multi { ref files, .. } => files.iter().fold(0, |a, h| a + h.total_bytes())
        }
    }
}

fn make_info_hash(val: &BencodeValue) -> BoostResult<[u8;20]> {
    //get dict from val
    if let &BencodeValue::Dict(ref d) = val {
        //get value associated with info key
        let &(_,ref info_dict) = d.iter().find(|&r| r.0 == "info".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find info dict")))?;
        let to_hash = info_dict.bencode();
        let mut hasher = Sha1::new();
        hasher.update(&to_hash.as_slice());
        Ok(hasher.digest().bytes())

    } else {
        Err(BoostError::BencodeValueErr(String::from("Value not a dictionary")))
    }
}

///gets the announce url from the metafile bdecoded values
fn parse_announce(val: &BencodeValue) -> BoostResult<String> {
    //get dict from val
    if let &BencodeValue::Dict(ref d) = val {
        //get value associated with announce key
        let announce_result = d.iter().find(|&r| r.0 == "announce".as_bytes());
        announce_result.map(|&(_,ref v)|  match v {
            &BencodeValue::Str(ref s) => String::from(str::from_utf8(s).unwrap()),
            _ => String::from("")
        }).ok_or(BoostError::BencodeValueErr(String::from("Could not find Announce")))
    } else {
        Err(BoostError::BencodeValueErr(String::from("Value not a dictionary")))
    }
}

///gets the piece length and the piece hashes from the metafile bdecoded values
fn parse_pieces(val: &BencodeValue) -> BoostResult<(u64, Vec<[u8;20]>)> {
    //get dict from val
    if let &BencodeValue::Dict(ref d) = val {
        //get peer dict
        let &(_,ref info_dict) = d.iter().find(|&r| r.0 == "info".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find info dict")))?;
        if let &BencodeValue::Dict(ref info) = info_dict {
            //get piece_len and pieces from peer dict
            let &(_, ref piece_len) = info.iter().find(|&r| r.0 == "piece length".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find piece length")))?;
            let &(_, ref pieces) = info.iter().find(|&r| r.0 == "pieces".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find piece hashes")))?;
            //ensure they are the correct types
            if let (&BencodeValue::Integer(len), &BencodeValue::Str(ref pieces)) = (piece_len,pieces) {
                let mut pos = 0;
                let mut piece_vec = Vec::new();
                //create fixed length array and copy 20 bytes from pieces string into it
                while pieces.get(pos) != None {
                    let mut hash : [u8;20] = [0;20];
                    for idx in 0..20 {
                        hash[idx] = pieces[idx];
                    }
                    piece_vec.push(hash);
                    pos+=20;
                }
                Ok((len as u64,  piece_vec))

            } else {
                Err(BoostError::BencodeValueErr(String::from("Piece length is not an int or Pieces is not a string")))
            }

        } else {
            Err(BoostError::BencodeValueErr(String::from("Peer key is not associated with a dictionary")))
        }
    } else {
        Err(BoostError::BencodeValueErr(String::from("Value not a dictionary")))
    }
}

fn parse_fileinfo(val: &BencodeValue) -> BoostResult<FileInfo> {
    //get dict from val
    if let &BencodeValue::Dict(ref d) = val {
        //get peer dict
        let &(_,ref info_dict) = d.iter().find(|&r| r.0 == "info".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find info dict")))?;
        if let &BencodeValue::Dict(ref info) = info_dict {
            //get file name
            let &(_, ref name) = info.iter().find(|&r| r.0 == "name".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find name")))?;
            let name = match name {
                &BencodeValue::Str(ref n) => n,
                _ => return Err(BoostError::BencodeValueErr(String::from("Name key not associated with a string")))
            };
            let filename = str::from_utf8(name).map_err(|_|
                                                        BoostError::BencodeValueErr(String::from("Could not convert name from bytes")))?;
            let filename = String::from(filename);
            //try to get file length
            let length = info.iter().find(|&r| r.0 == "length".as_bytes());
            //if length is found, single file mode
            if let Some(&(_,BencodeValue::Integer(filelength))) = length {
                let filelength = filelength as u64;
                Ok(FileInfo::Single { filename, filelength })
            }
            //multi file mode
            else {
                //get list of files
                let &(_,ref files) = info.iter().find(|&r| r.0 == "files".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find files dict")))?;
                if let &BencodeValue::List(ref files) = files {
                    let mut fileinfos = Vec::new();
                    //iterate over all files
                    for value in files.iter() {
                        if let &BencodeValue::Dict(ref f) = value {
                            let &(_, ref len) = f.iter().find(|&r| r.0 == "length".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find a file length")))?;
                            let &(_, ref path) = f.iter().find(|&r| r.0 == "path".as_bytes()).ok_or(BoostError::BencodeValueErr(String::from("Could not find a file path")))?;
                            if let (&BencodeValue::Integer(len), &BencodeValue::Str(ref path)) = (len, path)  {
                                let path = str::from_utf8(path).map_err(|_| BoostError::BencodeValueErr(String::from("Could not parse a file name from bytes")))?;
                                fileinfos.push(FileInfo::Single { filename: String::from(path), filelength: len as u64 });

                            } else {
                                return Err(BoostError::BencodeValueErr(String::from("Either len is not an integer of path is not a string")))
                            }

                        } else {
                            return Err(BoostError::BencodeValueErr(String::from("File not a dict")))
                        }
                    }
                    Ok(FileInfo::Multi { rootdir: filename, files: fileinfos })
                } else {
                    Err(BoostError::BencodeValueErr(String::from("Files key not associated with a dict")))
                }
            }

        } else {
            Err(BoostError::BencodeValueErr(String::from("Peer key is not associated with a dictionary")))
        }
    } else {
        Err(BoostError::BencodeValueErr(String::from("Value not a dictionary")))
    }
}
