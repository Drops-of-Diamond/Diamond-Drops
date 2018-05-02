// Adapted from Python from https://github.com/ethereum/py-evm/pull/555/files.

/* Started work on adapting to Rust manually below, but below that,
trying to use the Python code in Rust.

https://www.infoworld.com/article/3208391/python/how-rust-can-replace-c-with-pythons-help.html

PyO3 looks to be the most suitable:

https://github.com/PyO3/PyO3.

However it isn't clear how to use Python code in Rust, so I created an
iisue for that:
https://github.com/PyO3/pyo3/issues/148
*/

use modules::primitives::{Chunk};
// use ethereum_types::U256;
use modules::collation::constants::{CHUNK_SIZE, CHUNK_DATA_SIZE};

use bin::bin

use std::ops::{Generator, GeneratorState};

pub struct CollationVariables {
    collation_size: u32, // bytes
    chunks_per_collation: usize, // 2**15
    // size of a blob filling a full collation
    max_blob_size: usize,
}
/*
impl CollationVariables {
    // pow is not a constant function, so this can't be a constant or static.
    collation_size: 2_u32.pow(20), // bytes
    chunks_per_collation: collation_size / CHUNK_SIZE, // 2**15
    /// size of a blob filling a full collation
    max_blob_size: chunks_per_collation * CHUNK_DATA_SIZE,
}*/

// https://doc.rust-lang.org/book/second-edition/ch13-02-iterators.html#creating-our-own-iterators-with-iterator
#[derive(PartialEq, Debug, Clone)]
/// A collation body of 2^15 chunks with 32 bytes (256 bits) per chunk.
pub struct Body {
    body: [Chunk; bin::collation_variables.chunks_per_collation]
}
/*
pub impl Body {
    pub fn new() -> Body {
        Body {body: vec![[0x00, constants::CHUNK_SIZE], constants::chunks_per_collation]}
    }

    /*fn check_body_size(&self) -> <self::body, Err> {
        if self.body.len() != modules::collation::constants::collation_size {
            error!("{} is not equal to {}", self.body.len(), modules::collation::constants::collation_size)
            process::exit(1);
        }
    // I'm not sure if there's any advantage to writing it this way instead
    // of assert_eq!(self.body.len(), collation_size).
    }*/
}

impl Iterator for Body {
    type Item = [ethereum_types::U256, constants::chunks_per_collation];

    #![feature(iterator_step_by)] // nightly, for step_by()
    // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.step_by
    #![feature(generators, generator_trait)] // nightly, for yield
    // https://doc.rust-lang.org/std/ops/trait.Generator.html
    fn next(&mut self) -> Option<Self::Item> {
        //check_body_size(self.body)
        for chunk_start in (0..self.body.len()).step_by(CHUNK_SIZE) {
            let mut generator = || {
                yield self.body[chunk_start:chunk_start + CHUNK_SIZE];
                // Make this also return; we just want to return the last chunk.
            };

            match generator.resume() {
                GeneratorState::Yielded(self.body[chunk_start:chunk_start + CHUNK_SIZE]) => {}
                _ => ()//panic!("unexpected return from resume"),
            }
            match generator.resume() {
                GeneratorState::Complete(self.body[chunk_start:chunk_start + CHUNK_SIZE]) => {}
                _ => ()//panic!("unexpected return from resume"),
            }
        }
    }
}

/*
extern crate cpython;

use cpython::{Python, PyDict, PyResult};
*/
*/

#[cfg(test)]
mod tests {

    #[test]
    fn collation_size_mod_chunk_size_is_zero() {
        assert_eq!(collation_size % CHUNK_SIZE, 0, "Test: collation_size {} % 
            CHUNK_SIZE {} == 0", collation_size, CHUNK_SIZE);
    }
}