///A struct that holds a bitvector
pub struct BitVector {
    vec: Vec<u8>,
    num_bits: usize
}

impl BitVector {
    ///Creates a new bitvector with the specified number of bits
    pub fn new(num_bits: usize) -> Self {
        let mut bytes = num_bits / 8;
        if num_bits % 8 != 0 {
            bytes += 1;
        }
        BitVector { vec: Vec::with_capacity(bytes), num_bits }
    }

    ///Sets the given bit from a 0 to a 1
    pub fn set_index(&mut self, index: usize) {
        if index < self.num_bits {
            let byte = index / 8;
            let bit = index % 8;
            self.vec[byte] = 0x80 >> bit;
        }
    }

    ///checks if the given bit is a 0 or a 1
    pub fn index_isset(&self, index: usize) -> bool{
        if index < self.num_bits {
            let byte = index / 8;
            let bit = index % 8;
            self.vec[byte] & (0x80 >> bit) != 0
        } else {
            false
        }
    }

    ///checks if all the bits are 1
    pub fn is_complete(&self) -> bool {
        for index in 0..self.num_bits {
            if !self.index_isset(index) {
                return false
            }
        }
        true
    }

    ///clears all set 1s
    pub fn clear(&mut self) {
       for index in 0..self.vec.len() {
           self.vec[index] = 0;
       }
    }

    ///returns true if this bitvector and the other have the same length
    ///and some bits in common
    pub fn intersects(&self, other: &Self) -> bool {
        if self.num_bits == other.num_bits {
            for index in 0..self.vec.len() {
                if self.vec[index] & other.vec[index] != 0 {
                    return true
                }
            }
            false
        } else {
            false
       }
    }

    pub fn first_unset_index(&self) -> usize {
        for idx in 0..self.num_bits {
            if !self.index_isset(idx) {
                return idx
            }
        }
        self.num_bits
    }

    pub fn bit_len(&self) -> usize {
        self.num_bits
    }

    pub fn byte_len(&self) -> usize {
        self.vec.len()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.vec.as_slice()
    }
}
