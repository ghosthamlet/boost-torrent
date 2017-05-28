use std::fs::{File, create_dir_all, create_dir};
use std::io::prelude::*;
use std::io::{BufWriter, SeekFrom};
use meta::{MetaInfo, FileInfo};

///A struct that holds the reader and writer for the torrent file.
///Writes will be buffered until a read.
///Both Reads and Writes are random access
pub struct TorrentFile {
    file_reader: File,
    file_writer: BufWriter<File>,
    meta: MetaInfo
}

impl TorrentFile {
    ///Given a metainfo, creates a torrent file with all the space
    ///needed for the torrent
    pub fn init(meta: MetaInfo) -> Result<Self,String> {
        //working name is filename or root directory of multifile structure
        let working_name = match &meta.file_info {
            &FileInfo::Single {ref filename, ..} => filename.clone(),
            &FileInfo::Multi {ref rootdir,..} => rootdir.clone()
        };
        //working len is the file len or the sum of all the file len
        let working_len = match &meta.file_info {
            &FileInfo::Single {filelength,..} => filelength,
            &FileInfo::Multi {ref files, ..} => {
                let mut len = 0;
                for file in files {
                    if let &FileInfo::Single {filelength, ..} = file {
                        len += filelength;
                    } else {
                        return Err(format!("One of the files was itself a multifile"))
                    }
                }
                len
            }
        };
        //create the file
        let writer = File::create(&working_name).map_err(|_| "Could not create file")?;
        //set file size
        let _ = writer.set_len(working_len).map_err(|_| "Could not increas file len")?;
        //get buffered writer withbuffer capacity for 10 whole pieces
        let file_writer = BufWriter::with_capacity(meta.piece_len as usize * 10, writer);
        let file_reader = File::open(&working_name).map_err(|_| "Could not open file")?;

        Ok(TorrentFile { file_reader, file_writer, meta })
    }

    ///writes into the file from the buffer to the given offset in the file
    pub fn write(&mut self, offset: u64, buffer: &[u8]) {
        let _ = self.file_writer.seek(SeekFrom::Start(offset));
        let _ = self.file_writer.write(buffer);
    }

    ///reads into the buffer from the file at the given offset.
    pub fn read(&mut self, offset: u64, buffer: &mut [u8]) {
        let _ = self.file_writer.flush();
        let _ = self.file_reader.seek(SeekFrom::Start(offset));
        let _ = self.file_reader.read(buffer);
    }
}

impl Drop for TorrentFile {
    ///will turn the single torrent file into multiple files if the torrent is multifile
    fn drop(&mut self) {
        match &self.meta.file_info {
            &FileInfo::Single {..} => (), //single file, nothing to do
            &FileInfo::Multi { ref rootdir, ref files } => {
                let mut xbuf = [0u8;4096];
                let _ = self.file_reader.seek(SeekFrom::Start(0));
                //create root directory
                let _ = create_dir(rootdir) ;
                for file in files {
                   if let &FileInfo::Single { ref filename, filelength} = file {
                       //if the file is part of a path, make sure the entire path exists
                        if let Some(idx) = filename.rfind("/") {
                            let (path, file) = filename.split_at(idx);
                            let _ = create_dir_all(path);
                        }
                        //open up a new file
                        if let Ok(mut new_file) = File::create(rootdir.clone() + "/" + filename) {
                            let rem = (filelength % 4096) as usize;
                            //copy from master file to new file in 4096 byte blocks
                            for idx in 0..(filelength/4096) {
                                let _ = self.file_reader.read(&mut xbuf);
                                let _ = new_file.write(&xbuf);
                            }
                            //copy the last block
                            let _ = self.file_reader.read(&mut xbuf[0..rem]);
                            let _ = new_file.write(&xbuf[0..rem]);
                        }
                        //new file dropped here, closes
                   }
                }
            }
        }
    }
}
