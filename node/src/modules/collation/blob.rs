// Defined according to https://ethresear.ch/t/blob-serialisation/1705.
// Some stuff is commented out from ChosunOne's additions for comparison.
use bitreader::BitReader;
use modules::collation::chunk;
use modules::constants::{CHUNK_SIZE, CHUNK_DATA_SIZE, 
    COLLATION_SIZE, CHUNKS_PER_COLLATION, MAX_BLOB_SIZE};
//use std::convert::AsMut;

/* From trying to use an array instead of a vector in the data field
of chunks and blobs, but this is probably unnecessary.
/// Convert a slice of an array to a fixed size array.
/// From https://stackoverflow.com/a/37682288/7438857.
pub fn clone_into_array<A, T>(slice: &[T]) -> A
        where A: Sized + Default + AsMut<[T]>,
            T: Clone
    {
        let mut a = Default::default();
        <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
        a
    }
*/

/// Struct of a blob containing data of arbitrary size.
#[derive(PartialEq, Debug, Clone)]
pub struct Blob {
    data: Vec<u8>
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Blob {
        Blob  {
            data
        }
    }

    /// Create a set of chunks to represent this blob.
    pub fn to_chunks(self, skip_evm: bool) -> Vec<chunk::Chunk> {
        // Since each element is one byte then getting the length (the number of elements)
        // will also get the number of bytes.
        let bytes_per_blob: usize = self.data.len();
        // We can only fit 31 (CHUNK_DATA_SIZE) out of 32 (CHUNK_SIZE) bytes
        // per blob into a chunk, due to adding the indicator byte.
        // self.size is the number of bytes per blob
        // Dimensional analysis:
        // chunks_per_blob = [bytes per blob] / [blob bytes that fit into each chunk].
        let chunks_per_blob: usize = (((bytes_per_blob as f32 / CHUNK_DATA_SIZE as f32) 
            as f32).ceil()) as usize;
        // E.g.: for a 125 byte blob at 31 bytes per chunk, that's 124 bytes that fit 
        // into 4 chunks, plus the last chunk for the last byte, i.e. 5 chunks. 
        // 125 / 31 rounded up is 5.
        let terminal_len: u8 = (bytes_per_blob % CHUNK_DATA_SIZE) as u8;
        // 125 % 31 = 1.
        // For collecting chunks:
        let mut chunks: Vec<chunk::Chunk> = vec![];
        // Iterate, collecting the blob into chunks
        // Remember, for loops don't iterate the last step in a range!
        // So start from 0 to add 1 (and to avoid doing operations for performance)
        for i in 0..chunks_per_blob {
            // 0, 1, 2, ... chunks_per_blob - 1
            let mut indicator: u8 = 0b0000_0000;
            let mut ch: chunk::Chunk;
            let i_data_start: usize = (i * CHUNK_DATA_SIZE) as usize;
            // 0, 31, 62, ... chunks_per_blob * CHUNK_DATA_SIZE
            let mut chunk_data: [u8; CHUNK_DATA_SIZE] = [0; CHUNK_DATA_SIZE];
            if i != chunks_per_blob { 
                // Build non-terminal chunks
                // i_data_end_plus_1 = i_data_start_of_next_31_bytes
                let i_data_end_plus_1: usize = ((i + 1) * CHUNK_DATA_SIZE) as usize;
                // 31, 62, 93, ..., (chunks_per_blob + 1) * CHUNK_DATA_SIZE
                // Again, it doesn't do the last step.
                for j in i_data_start..(i_data_end_plus_1) {
                    // j = (loop 1) 0..30, (loop 2), 31..61, 
                    // 62..92, ..., (chunks_per_blob - 2)*CHUNK_DATA_SIZE
                    // .. (chunks_per_blob - 1) * CHUNK_DATA_SIZE - 1
                    // chunks_per_blob won't run in this if loop, it will go up to 
                    // chunks_per_blob - 1, which will then not go into the block.
                    chunk_data[j - i_data_start] = self.data[j];
                }
            } else {
                // Build the terminal chunk
                // Set the 5 least significant bits of the indicator byte
                indicator += terminal_len;
                // Set the chunk data, saving iterating the last 0 consecutive bits of
                // the terminal chunk, which are set to 0 anyway above.
                for j in i_data_start..bytes_per_blob {
                    chunk_data[j - i_data_start] = self.data[j];
                }
            }
            // Set the first bit of the indicator byte, the skip_evm flag,
            // if a skip_evm opcode was called.
            // This will be false until there are multiple EVMs.
            // The skip_evm opcode could change the value of skip_evm to true
            // Should also separate this into a separate function and use a closure
            // to avoid repeating the same code twice
            if skip_evm {
                // Set SKIP_EVM flag to 1
                indicator += 0b1000_0000;
            }
            ch = chunk::Chunk::new(indicator, chunk_data);
            chunks.push(ch);
        }
        chunks
    }
    /// Create a blob from a set of chunks
    pub fn from_chunks(chunks: Vec<chunk::Chunk>) -> Blob {
        let mut data = vec![];
        for ch in chunks {
            // read the first 3 bits of the indicator and discard the result.
            BitReader::new(&[ch.indicator]).read_u8(3).unwrap();
            // read the remaining 5 bits into length bits
            let mut data_bytes_length_in_terminal_chunk 
                = BitReader::new(&[ch.indicator]).read_u8(5).unwrap();
            if data_bytes_length_in_terminal_chunk == 0b0_0000 {
                // Chunk is not terminal, read all CHUNK_DATA_SIZE bytes into data.
                for i in 0..CHUNK_DATA_SIZE {
                    data.push(ch.data[i as usize]);
                }
            } else {                
                // Chunk is terminal, read data_bytes_length_in_terminal_chunk bytes into data
                for i in 0..data_bytes_length_in_terminal_chunk {
                    data.push(ch.data[i as usize]);
                }
            }
        }
        Blob {
            data
        }
    }
}

/// Put blob chunks into (a) collation body(ies).
/* if to_chunks(blob).length > CHUNKS_PER_COLLATION {
    Serialize a blob into multiple collation bodies.
} else {
    Pack the blob chunks into the collation body.
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_converts_to_chunks_with_skip_evm_false_and_a_small_blob() {
        let blob = Blob::new(vec![0xff; 4]);
        let blob_chunks = blob.to_chunks(false);
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(false, true, 4);
        let mut correct_blob_chunks = vec![chunk::Chunk::new(terminal_chunk_indicator, 
                                                                [0xff, 0xff, 0xff, 0xff,
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0])];
        assert_eq!(blob_chunks, correct_blob_chunks);
    }

    #[test]
    fn it_converts_to_chunks_with_skip_evm_false_and_a_32_byte_blob() {
        let blob = Blob::new(vec![0xff; 32]);
        let blob_chunks = blob.to_chunks(false);
        let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(false, false, 0);
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(false, true, 1);
        let mut correct_blob_chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator, 
                                                                [0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff, 0xff,
                                                                0xff, 0xff, 0xff])];
        correct_blob_chunks.push(chunk::Chunk::new(terminal_chunk_indicator, [0xff,
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));                                                                                      
        assert_eq!(blob_chunks, correct_blob_chunks);
    }

    #[test]
    fn it_converts_to_chunks_with_skip_evm_false() {
        let blob = Blob::new(vec![0xff; CHUNK_DATA_SIZE*4+4]);
        let blob_chunks = blob.to_chunks(false);
        let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(false, false, 0);
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(false, true, 4);
        let mut correct_blob_chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator, 
            [0xff; CHUNK_DATA_SIZE]); 4];
        correct_blob_chunks.push(chunk::Chunk::new(terminal_chunk_indicator, 
                                                                [0xff, 0xff, 0xff, 0xff,
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert_eq!(blob_chunks, correct_blob_chunks);
    }

    #[test]
    fn it_converts_to_chunks_with_skip_evm_true() {
        let blob = Blob::new(vec![0xff; CHUNK_DATA_SIZE*4+4]);
        let blob_chunks = blob.to_chunks(true);
        let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(true, false, 0);
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(true, true, 4);
        let mut correct_blob_chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator, 
            [0xff; CHUNK_DATA_SIZE]); 4];
        correct_blob_chunks.push(chunk::Chunk::new(terminal_chunk_indicator, 
                                                                [0xff, 0xff, 0xff, 0xff, 
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0,
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 
                                                                0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert_eq!(blob_chunks, correct_blob_chunks);
    }

    #[test]
    fn it_converts_from_chunks_with_skip_evm_false_and_a_small_blob() {
        // 0b0000_0000
        //let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(false, false, 0);
        // 0b0001_1111
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(
            false, true, 4);
        // 4 chunks, with every non-indicator byte as 255
        //let mut chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator,
        //   [255; CHUNK_DATA_SIZE]); 4];
        // 5th terminal chunk, also with every non-indicator byte as 255
        let mut chunk_1_data = [0; 31]; 
        for i in 0..4 {
            chunk_1_data[i] = 255;
        }
        // Alternative: https://play.rust-lang.org/?gist=53969b6c3cad694d219091e8550c8ccc&version=undefined&mode=undefined
        // Less efficient: https://play.rust-lang.org/?gist=8ca17ca652ab781b5c4e1fe14ef4f919&version=stable&mode=debug
        let mut chunks = vec![chunk::Chunk::new(terminal_chunk_indicator, chunk_1_data)];
        let blob_from_chunks = Blob::from_chunks(chunks);
        let blob = Blob::new(chunk_1_data.to_vec());
        assert_eq!(blob, blob_from_chunks);
    }

    #[test]
    fn it_converts_from_chunks_with_skip_evm_false_and_a_32_byte_blob() {
        // 0b0000_0000
        let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(false, false, 0);
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(
            false, true, 1 as u8);
        // 1 chunks, with every non-indicator byte as 255
        let mut chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator,
            [255; CHUNK_DATA_SIZE])];
        // 2nd terminal chunk, with one byte as 255.
        let mut chunk_2_data = [0; 31]; chunk_2_data[0] = 255;
        chunks.push(chunk::Chunk::new(terminal_chunk_indicator, chunk_2_data));
        let blob_from_chunks = Blob::from_chunks(chunks);
        let mut blob = Blob::new(vec![0; 62]);
        for i in 0..32 {
            blob.data[i] = 255;
        }
        assert_eq!(blob, blob_from_chunks);
    }

    #[test]
    fn it_converts_from_chunks_with_skip_evm_false() {
        // 0b0000_0000
        let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(false, false, 0);
        // 0b0001_1111
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(
            false, true, CHUNK_DATA_SIZE as u8);
        // 4 chunks, with every non-indicator byte as 255
        let mut chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator,
            [255; CHUNK_DATA_SIZE]); 4];
        // 5th terminal chunk, also with every non-indicator byte as 255
        chunks.push(chunk::Chunk::new(terminal_chunk_indicator, [255; CHUNK_DATA_SIZE]));
        let blob_from_chunks = Blob::from_chunks(chunks);
        let blob = Blob::new(vec![255; CHUNK_DATA_SIZE*5]);
        assert_eq!(blob, blob_from_chunks);
    }

    #[test]
    fn it_converts_from_chunks_with_skip_evm_true() {
        // 0b1000_0000
        let non_terminal_chunk_indicator = chunk::Chunk::build_indicator(true, false, 0);
        // 0b1001_1111
        let terminal_chunk_indicator = chunk::Chunk::build_indicator(
            true, true, CHUNK_DATA_SIZE as u8);
        // as above: 4 chunks, with every non-indicator byte as 255
        let mut chunks = vec![chunk::Chunk::new(non_terminal_chunk_indicator,  
            [255; CHUNK_DATA_SIZE]); 4];
        // ditto: 5th terminal chunk, also with every non-indicator byte as 255
        chunks.push(chunk::Chunk::new(terminal_chunk_indicator, [255; CHUNK_DATA_SIZE]));
        let blob_from_chunks = Blob::from_chunks(chunks);
        let blob = Blob::new(vec![255; CHUNK_DATA_SIZE*5]);
        assert_eq!(blob, blob_from_chunks);
    }
}