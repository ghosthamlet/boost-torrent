use bitvector::BitVector;
use std::time::{Instant, Duration};
use sha1::Sha1;
use message::BitTorrentMessage;

const BLOCK_SIZE: u32 = 16384; //2^14
const REQUESTS_STALE_AFTER_MILLIS: u64 = 500; //.5 seconds

///A struct for holding piece data while its being worked on
pub struct Piece {
    index: u32,
    piece_size: u32,
    obtained_blocks: BitVector,
    requested_blocks: BitVector,
    last_updated: Instant,
    piece: Vec<u8>,
    hash: [u8; 20]
}

impl Piece {

    ///creates a new piece. Index should be the 0-based index of this piece
    ///in the completed pieces bitvector. piece_size should be the total size
    ///of this whole piece in bytes. This should be the same for all pieces
    ///except for the last piece which may or may not be shorter than the rest.
    ///hash is what the piece will be verified against when it is completed.
    pub fn new(index: u32, piece_size: u32, hash: [u8;20]) -> Self {
        let extra = if piece_size % BLOCK_SIZE == 0 { 0 } else { 1 };
        let num_blocks = piece_size / BLOCK_SIZE + extra;
        Piece {
            index,
            piece_size,
            obtained_blocks: BitVector::new(num_blocks as usize),
            requested_blocks: BitVector::new(num_blocks as usize),
            last_updated: Instant::now(),
            piece: Vec::with_capacity(piece_size as usize),
            hash: hash
        }
    }

    ///Returns whether all blocks have been retrieved for this piece
    pub fn is_complete(&self) -> bool {
        self.obtained_blocks.is_complete()
    }

    ///returns whether this piece is valid or not
    pub fn is_correct(&self) -> bool {
        let mut hasher = Sha1::new();
        hasher.update(self.piece.as_slice());
        hasher.digest().bytes() == self.hash
    }

    ///returns a request message for the next desired piece
    pub fn next_request(&mut self) -> Option<BitTorrentMessage> {
        if self.is_complete() {
            None
        } else {
            //if this piece hasn't been updated in a while, reset all pending requests
            if self.is_requests_stale() {
                self.requested_blocks.clear();
                for idx in 0..self.requested_blocks.bit_len() {
                    if self.obtained_blocks.index_isset(idx) {
                        self.requested_blocks.set_index(idx)
                    }
                }
            };
            //get the first unrequested block
            let block_idx = self.requested_blocks.first_unset_index() as u32;
            let block_begin = block_idx * BLOCK_SIZE;
            //get block size, either the predetermined size or the last block size, which may be
            //smaller
            let length = if self.piece_size - block_begin < BLOCK_SIZE {
                self.piece_size - block_begin
            } else {
                BLOCK_SIZE
            };
            self.last_updated = Instant::now();
            self.requested_blocks.set_index(block_idx as usize);
            Some(BitTorrentMessage::Request { piece_index: self.index, begin: block_begin, length })
        }
    }

    ///Takes a block and an offset and updates this piece
    pub fn add_block(&mut self, block_offset: u32, block: &Vec<u8>) {
       self.last_updated = Instant::now();
       self.obtained_blocks.set_index((block_offset / BLOCK_SIZE) as usize);
       let mut off = block_offset as usize;
       for byte in block {
           self.piece[off] = *byte;
           off += 1;
       }
    }

    fn is_requests_stale(&self) -> bool {
        Instant::now() - self.last_updated > Duration::from_millis(REQUESTS_STALE_AFTER_MILLIS)
    }

}
