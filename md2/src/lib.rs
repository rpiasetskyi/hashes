//! An implementation of the [MD2][1] cryptographic hash algorithm.
//!
//! # Usage
//!
//! ```rust
//! use md2::{Md2, Digest};
//! use hex_literal::hex;
//!
//! // create a Md2 hasher instance
//! let mut hasher = Md2::new();
//!
//! // process input message
//! hasher.update(b"hello world");
//!
//! // acquire hash digest in the form of GenericArray,
//! // which in this case is equivalent to [u8; 16]
//! let result = hasher.finalize();
//! assert_eq!(result[..], hex!("d9cce882ee690a5c1ce70beff3a78c77"));
//! ```
//!
//! Also see [RustCrypto/hashes][2] readme.
//!
//! [1]: https://en.wikipedia.org/wiki/MD4
//! [2]: https://github.com/RustCrypto/hashes

#![no_std]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RustCrypto/media/6ee8e381/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/RustCrypto/media/6ee8e381/logo.svg"
)]
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub use digest::{self, Digest};

use core::fmt;
use digest::{
    block_buffer::Eager,
    consts::{U16, U48, U64},
    core_api::{
        AlgorithmName, Block, BlockSizeUser, Buffer, BufferKindUser, CoreWrapper, FixedOutputCore,
        OutputSizeUser, Reset, UpdateCore,
    },
    crypto_common::{DeserializeStateError, SerializableState, SerializedState},
    generic_array::{
        sequence::{Concat, Split},
        GenericArray,
    },
    HashMarker, Output,
};

mod consts;

/// Core MD2 hasher state.
#[derive(Clone)]
pub struct Md2Core {
    x: [u8; STATE_LEN],
    checksum: Block<Self>,
}

const STATE_LEN: usize = 48;

impl Md2Core {
    fn compress(&mut self, block: &Block<Self>) {
        // Update state
        for j in 0..16 {
            self.x[16 + j] = block[j];
            self.x[32 + j] = self.x[16 + j] ^ self.x[j];
        }

        let mut t = 0u8;
        for j in 0..18u8 {
            for k in 0..STATE_LEN {
                self.x[k] ^= consts::S[t as usize];
                t = self.x[k];
            }
            t = t.wrapping_add(j);
        }

        // Update checksum
        let mut l = self.checksum[15];
        for j in 0..16 {
            self.checksum[j] ^= consts::S[(block[j] ^ l) as usize];
            l = self.checksum[j];
        }
    }
}

impl HashMarker for Md2Core {}

impl BlockSizeUser for Md2Core {
    type BlockSize = U16;
}

impl BufferKindUser for Md2Core {
    type BufferKind = Eager;
}

impl OutputSizeUser for Md2Core {
    type OutputSize = U16;
}

impl UpdateCore for Md2Core {
    #[inline]
    fn update_blocks(&mut self, blocks: &[Block<Self>]) {
        for block in blocks {
            self.compress(block)
        }
    }
}

impl FixedOutputCore for Md2Core {
    #[inline]
    fn finalize_fixed_core(&mut self, buffer: &mut Buffer<Self>, out: &mut Output<Self>) {
        let pos = buffer.get_pos();
        let rem = buffer.remaining() as u8;
        let block = buffer.pad_with_zeros();
        block[pos..].iter_mut().for_each(|b| *b = rem);

        self.compress(block);
        let checksum = self.checksum;
        self.compress(&checksum);
        out.copy_from_slice(&self.x[0..16]);
    }
}

impl Default for Md2Core {
    #[inline]
    fn default() -> Self {
        Self {
            x: [0; STATE_LEN],
            checksum: Default::default(),
        }
    }
}

impl Reset for Md2Core {
    #[inline]
    fn reset(&mut self) {
        *self = Default::default();
    }
}

impl AlgorithmName for Md2Core {
    fn write_alg_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Md2")
    }
}

impl fmt::Debug for Md2Core {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Md2Core { ... }")
    }
}

impl SerializableState for Md2Core {
    type SerializedStateSize = U64;

    fn serialize(&self) -> SerializedState<Self> {
        GenericArray::<_, U48>::from(self.x).concat(self.checksum)
    }

    fn deserialize(
        serialized_state: &SerializedState<Self>,
    ) -> Result<Self, DeserializeStateError> {
        let (serialized_x, serialized_checksum) = Split::<_, U48>::split(serialized_state);

        Ok(Self {
            x: (*serialized_x).into(),
            checksum: *serialized_checksum,
        })
    }
}

/// MD2 hasher state.
pub type Md2 = CoreWrapper<Md2Core>;
