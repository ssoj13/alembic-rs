//! SpookyHash V2 - A 128-bit non-cryptographic hash function by Bob Jenkins.
//!
//! This is a direct port of the SpookyV2 implementation from Alembic,
//! ensuring binary compatibility with Alembic files.
//!
//! # Example
//! ```
//! use spooky_hash::SpookyHash;
//!
//! // One-shot hashing
//! let (h1, h2) = SpookyHash::hash128(b"hello world", 0, 0);
//!
//! // Incremental hashing
//! let mut hasher = SpookyHash::new(0, 0);
//! hasher.update(b"hello ");
//! hasher.update(b"world");
//! let (h1, h2) = hasher.finalize();
//! ```

#![cfg_attr(not(test), no_std)]

/// SpookyHash V2 constants
const SC_CONST: u64 = 0xdeadbeefdeadbeef;
const SC_NUM_VARS: usize = 12;
const SC_BLOCK_SIZE: usize = SC_NUM_VARS * 8; // 96 bytes
const SC_BUF_SIZE: usize = 2 * SC_BLOCK_SIZE; // 192 bytes

/// Left-rotate a 64-bit value by k bits
#[inline(always)]
const fn rot64(x: u64, k: u32) -> u64 {
    x.rotate_left(k)
}

/// Mix function for long messages (96+ bytes per block)
#[inline(always)]
fn mix(
    data: &[u64; SC_NUM_VARS],
    s: &mut [u64; SC_NUM_VARS],
) {
    s[0] = s[0].wrapping_add(data[0]);   s[2] ^= s[10];  s[11] ^= s[0];   s[0] = rot64(s[0], 11);  s[11] = s[11].wrapping_add(s[1]);
    s[1] = s[1].wrapping_add(data[1]);   s[3] ^= s[11];  s[0] ^= s[1];    s[1] = rot64(s[1], 32);  s[0] = s[0].wrapping_add(s[2]);
    s[2] = s[2].wrapping_add(data[2]);   s[4] ^= s[0];   s[1] ^= s[2];    s[2] = rot64(s[2], 43);  s[1] = s[1].wrapping_add(s[3]);
    s[3] = s[3].wrapping_add(data[3]);   s[5] ^= s[1];   s[2] ^= s[3];    s[3] = rot64(s[3], 31);  s[2] = s[2].wrapping_add(s[4]);
    s[4] = s[4].wrapping_add(data[4]);   s[6] ^= s[2];   s[3] ^= s[4];    s[4] = rot64(s[4], 17);  s[3] = s[3].wrapping_add(s[5]);
    s[5] = s[5].wrapping_add(data[5]);   s[7] ^= s[3];   s[4] ^= s[5];    s[5] = rot64(s[5], 28);  s[4] = s[4].wrapping_add(s[6]);
    s[6] = s[6].wrapping_add(data[6]);   s[8] ^= s[4];   s[5] ^= s[6];    s[6] = rot64(s[6], 39);  s[5] = s[5].wrapping_add(s[7]);
    s[7] = s[7].wrapping_add(data[7]);   s[9] ^= s[5];   s[6] ^= s[7];    s[7] = rot64(s[7], 57);  s[6] = s[6].wrapping_add(s[8]);
    s[8] = s[8].wrapping_add(data[8]);   s[10] ^= s[6];  s[7] ^= s[8];    s[8] = rot64(s[8], 55);  s[7] = s[7].wrapping_add(s[9]);
    s[9] = s[9].wrapping_add(data[9]);   s[11] ^= s[7];  s[8] ^= s[9];    s[9] = rot64(s[9], 54);  s[8] = s[8].wrapping_add(s[10]);
    s[10] = s[10].wrapping_add(data[10]); s[0] ^= s[8];  s[9] ^= s[10];   s[10] = rot64(s[10], 22); s[9] = s[9].wrapping_add(s[11]);
    s[11] = s[11].wrapping_add(data[11]); s[1] ^= s[9];  s[10] ^= s[11];  s[11] = rot64(s[11], 46); s[10] = s[10].wrapping_add(s[0]);
}

/// EndPartial - mix all 12 inputs for final hash
#[inline(always)]
fn end_partial(h: &mut [u64; SC_NUM_VARS]) {
    h[11] = h[11].wrapping_add(h[1]);  h[2] ^= h[11];  h[1] = rot64(h[1], 44);
    h[0] = h[0].wrapping_add(h[2]);    h[3] ^= h[0];   h[2] = rot64(h[2], 15);
    h[1] = h[1].wrapping_add(h[3]);    h[4] ^= h[1];   h[3] = rot64(h[3], 34);
    h[2] = h[2].wrapping_add(h[4]);    h[5] ^= h[2];   h[4] = rot64(h[4], 21);
    h[3] = h[3].wrapping_add(h[5]);    h[6] ^= h[3];   h[5] = rot64(h[5], 38);
    h[4] = h[4].wrapping_add(h[6]);    h[7] ^= h[4];   h[6] = rot64(h[6], 33);
    h[5] = h[5].wrapping_add(h[7]);    h[8] ^= h[5];   h[7] = rot64(h[7], 10);
    h[6] = h[6].wrapping_add(h[8]);    h[9] ^= h[6];   h[8] = rot64(h[8], 13);
    h[7] = h[7].wrapping_add(h[9]);    h[10] ^= h[7];  h[9] = rot64(h[9], 38);
    h[8] = h[8].wrapping_add(h[10]);   h[11] ^= h[8];  h[10] = rot64(h[10], 53);
    h[9] = h[9].wrapping_add(h[11]);   h[0] ^= h[9];   h[11] = rot64(h[11], 42);
    h[10] = h[10].wrapping_add(h[0]);  h[1] ^= h[10];  h[0] = rot64(h[0], 54);
}

/// End - final mixing with data
#[inline(always)]
fn end(data: &[u64; SC_NUM_VARS], h: &mut [u64; SC_NUM_VARS]) {
    for i in 0..SC_NUM_VARS {
        h[i] = h[i].wrapping_add(data[i]);
    }
    end_partial(h);
    end_partial(h);
    end_partial(h);
}

/// ShortMix - mixing for short messages
#[inline(always)]
fn short_mix(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
    *c = rot64(*c, 50); *c = c.wrapping_add(*d); *a ^= *c;
    *d = rot64(*d, 52); *d = d.wrapping_add(*a); *b ^= *d;
    *a = rot64(*a, 30); *a = a.wrapping_add(*b); *c ^= *a;
    *b = rot64(*b, 41); *b = b.wrapping_add(*c); *d ^= *b;
    *c = rot64(*c, 54); *c = c.wrapping_add(*d); *a ^= *c;
    *d = rot64(*d, 48); *d = d.wrapping_add(*a); *b ^= *d;
    *a = rot64(*a, 38); *a = a.wrapping_add(*b); *c ^= *a;
    *b = rot64(*b, 37); *b = b.wrapping_add(*c); *d ^= *b;
    *c = rot64(*c, 62); *c = c.wrapping_add(*d); *a ^= *c;
    *d = rot64(*d, 34); *d = d.wrapping_add(*a); *b ^= *d;
    *a = rot64(*a, 5);  *a = a.wrapping_add(*b); *c ^= *a;
    *b = rot64(*b, 36); *b = b.wrapping_add(*c); *d ^= *b;
}

/// ShortEnd - final mixing for short messages
#[inline(always)]
fn short_end(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
    *d ^= *c; *c = rot64(*c, 15); *d = d.wrapping_add(*c);
    *a ^= *d; *d = rot64(*d, 52); *a = a.wrapping_add(*d);
    *b ^= *a; *a = rot64(*a, 26); *b = b.wrapping_add(*a);
    *c ^= *b; *b = rot64(*b, 51); *c = c.wrapping_add(*b);
    *d ^= *c; *c = rot64(*c, 28); *d = d.wrapping_add(*c);
    *a ^= *d; *d = rot64(*d, 9);  *a = a.wrapping_add(*d);
    *b ^= *a; *a = rot64(*a, 47); *b = b.wrapping_add(*a);
    *c ^= *b; *b = rot64(*b, 54); *c = c.wrapping_add(*b);
    *d ^= *c; *c = rot64(*c, 32); *d = d.wrapping_add(*c);
    *a ^= *d; *d = rot64(*d, 25); *a = a.wrapping_add(*d);
    *b ^= *a; *a = rot64(*a, 63); *b = b.wrapping_add(*a);
}

/// Read u64 from bytes (little-endian)
#[inline(always)]
fn read_u64_le(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let len = bytes.len().min(8);
    buf[..len].copy_from_slice(&bytes[..len]);
    u64::from_le_bytes(buf)
}

/// Read u32 from bytes (little-endian)
#[inline(always)]
fn read_u32_le(bytes: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    let len = bytes.len().min(4);
    buf[..len].copy_from_slice(&bytes[..len]);
    u32::from_le_bytes(buf)
}

/// Short hash for messages < 192 bytes
fn hash_short(message: &[u8], hash1: u64, hash2: u64) -> (u64, u64) {
    let length = message.len();
    
    let mut a = hash1;
    let mut b = hash2;
    let mut c = SC_CONST;
    let mut d = SC_CONST;
    
    let mut offset = 0;
    
    if length > 15 {
        // Handle complete 32-byte blocks
        let end = (length / 32) * 32;
        while offset < end {
            c = c.wrapping_add(read_u64_le(&message[offset..]));
            d = d.wrapping_add(read_u64_le(&message[offset + 8..]));
            short_mix(&mut a, &mut b, &mut c, &mut d);
            a = a.wrapping_add(read_u64_le(&message[offset + 16..]));
            b = b.wrapping_add(read_u64_le(&message[offset + 24..]));
            offset += 32;
        }
        
        // Handle 16+ remaining bytes
        let remaining = length - offset;
        if remaining >= 16 {
            c = c.wrapping_add(read_u64_le(&message[offset..]));
            d = d.wrapping_add(read_u64_le(&message[offset + 8..]));
            short_mix(&mut a, &mut b, &mut c, &mut d);
            offset += 16;
        }
    }
    
    // Handle last 0..15 bytes and length
    d = d.wrapping_add((length as u64) << 56);
    let remaining = length - offset;
    let tail = &message[offset..];
    
    match remaining {
        15 => {
            d = d.wrapping_add((tail[14] as u64) << 48);
            d = d.wrapping_add((tail[13] as u64) << 40);
            d = d.wrapping_add((tail[12] as u64) << 32);
            d = d.wrapping_add(read_u32_le(&tail[8..]) as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        14 => {
            d = d.wrapping_add((tail[13] as u64) << 40);
            d = d.wrapping_add((tail[12] as u64) << 32);
            d = d.wrapping_add(read_u32_le(&tail[8..]) as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        13 => {
            d = d.wrapping_add((tail[12] as u64) << 32);
            d = d.wrapping_add(read_u32_le(&tail[8..]) as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        12 => {
            d = d.wrapping_add(read_u32_le(&tail[8..]) as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        11 => {
            d = d.wrapping_add((tail[10] as u64) << 16);
            d = d.wrapping_add((tail[9] as u64) << 8);
            d = d.wrapping_add(tail[8] as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        10 => {
            d = d.wrapping_add((tail[9] as u64) << 8);
            d = d.wrapping_add(tail[8] as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        9 => {
            d = d.wrapping_add(tail[8] as u64);
            c = c.wrapping_add(read_u64_le(tail));
        }
        8 => {
            c = c.wrapping_add(read_u64_le(tail));
        }
        7 => {
            c = c.wrapping_add((tail[6] as u64) << 48);
            c = c.wrapping_add((tail[5] as u64) << 40);
            c = c.wrapping_add((tail[4] as u64) << 32);
            c = c.wrapping_add(read_u32_le(tail) as u64);
        }
        6 => {
            c = c.wrapping_add((tail[5] as u64) << 40);
            c = c.wrapping_add((tail[4] as u64) << 32);
            c = c.wrapping_add(read_u32_le(tail) as u64);
        }
        5 => {
            c = c.wrapping_add((tail[4] as u64) << 32);
            c = c.wrapping_add(read_u32_le(tail) as u64);
        }
        4 => {
            c = c.wrapping_add(read_u32_le(tail) as u64);
        }
        3 => {
            c = c.wrapping_add((tail[2] as u64) << 16);
            c = c.wrapping_add((tail[1] as u64) << 8);
            c = c.wrapping_add(tail[0] as u64);
        }
        2 => {
            c = c.wrapping_add((tail[1] as u64) << 8);
            c = c.wrapping_add(tail[0] as u64);
        }
        1 => {
            c = c.wrapping_add(tail[0] as u64);
        }
        0 => {
            c = c.wrapping_add(SC_CONST);
            d = d.wrapping_add(SC_CONST);
        }
        _ => unreachable!(),
    }
    
    short_end(&mut a, &mut b, &mut c, &mut d);
    (a, b)
}

/// SpookyHash V2 hasher for incremental hashing
pub struct SpookyHash {
    data: [u64; 2 * SC_NUM_VARS], // buffer for unhashed data
    state: [u64; SC_NUM_VARS],    // internal state
    length: usize,                // total length so far
    remainder: usize,             // length of buffered data
}

impl SpookyHash {
    /// Create a new SpookyHash with the given seeds
    pub fn new(seed1: u64, seed2: u64) -> Self {
        let mut state = [0u64; SC_NUM_VARS];
        state[0] = seed1;
        state[1] = seed2;
        Self {
            data: [0u64; 2 * SC_NUM_VARS],
            state,
            length: 0,
            remainder: 0,
        }
    }
    
    /// Update the hash with more data
    pub fn update(&mut self, message: &[u8]) {
        let new_length = message.len() + self.remainder;
        
        // If message fragment is too short, buffer it
        if new_length < SC_BUF_SIZE {
            let data_bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    self.data.as_mut_ptr() as *mut u8,
                    SC_BUF_SIZE * 2,
                )
            };
            data_bytes[self.remainder..self.remainder + message.len()]
                .copy_from_slice(message);
            self.length += message.len();
            self.remainder = new_length;
            return;
        }
        
        // Initialize state if this is first real processing
        let mut h = if self.length < SC_BUF_SIZE {
            [
                self.state[0], self.state[1], SC_CONST,
                self.state[0], self.state[1], SC_CONST,
                self.state[0], self.state[1], SC_CONST,
                self.state[0], self.state[1], SC_CONST,
            ]
        } else {
            self.state
        };
        
        self.length += message.len();
        let mut msg_offset = 0;
        
        // Process buffered data first
        if self.remainder > 0 {
            let prefix = SC_BUF_SIZE - self.remainder;
            let data_bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    self.data.as_mut_ptr() as *mut u8,
                    SC_BUF_SIZE * 2,
                )
            };
            data_bytes[self.remainder..self.remainder + prefix]
                .copy_from_slice(&message[..prefix]);
            
            // Mix both blocks from buffer
            let block1 = read_block(&self.data[..SC_NUM_VARS]);
            mix(&block1, &mut h);
            let block2 = read_block(&self.data[SC_NUM_VARS..]);
            mix(&block2, &mut h);
            
            msg_offset = prefix;
        }
        
        // Process whole blocks from message
        let msg = &message[msg_offset..];
        let num_blocks = msg.len() / SC_BLOCK_SIZE;
        
        for i in 0..num_blocks {
            let block_start = i * SC_BLOCK_SIZE;
            let block = read_block_from_bytes(&msg[block_start..]);
            mix(&block, &mut h);
        }
        
        // Buffer remaining bytes
        let processed = num_blocks * SC_BLOCK_SIZE;
        self.remainder = msg.len() - processed;
        
        if self.remainder > 0 {
            let data_bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    self.data.as_mut_ptr() as *mut u8,
                    SC_BUF_SIZE * 2,
                )
            };
            data_bytes[..self.remainder]
                .copy_from_slice(&msg[processed..]);
        }
        
        self.state = h;
    }
    
    /// Finalize and return the 128-bit hash
    pub fn finalize(&self) -> (u64, u64) {
        // Short message path
        if self.length < SC_BUF_SIZE {
            let data_bytes = unsafe {
                core::slice::from_raw_parts(
                    self.data.as_ptr() as *const u8,
                    self.length,
                )
            };
            return hash_short(data_bytes, self.state[0], self.state[1]);
        }
        
        let mut h = self.state;
        let remainder = self.remainder;
        
        // Handle first block if remainder >= 96
        let data_offset = if remainder >= SC_BLOCK_SIZE {
            let block = read_block(&self.data[..SC_NUM_VARS]);
            mix(&block, &mut h);
            SC_NUM_VARS
        } else {
            0
        };
        
        let final_remainder = if remainder >= SC_BLOCK_SIZE {
            remainder - SC_BLOCK_SIZE
        } else {
            remainder
        };
        
        // Prepare final block
        let mut buf = [0u64; SC_NUM_VARS];
        let buf_bytes = unsafe {
            core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, SC_BLOCK_SIZE)
        };
        
        let data_bytes = unsafe {
            core::slice::from_raw_parts(
                self.data[data_offset..].as_ptr() as *const u8,
                final_remainder,
            )
        };
        buf_bytes[..final_remainder].copy_from_slice(data_bytes);
        buf_bytes[SC_BLOCK_SIZE - 1] = final_remainder as u8;
        
        end(&buf, &mut h);
        
        (h[0], h[1])
    }
    
    /// Hash a message in one call, returning 128-bit hash
    pub fn hash128(message: &[u8], seed1: u64, seed2: u64) -> (u64, u64) {
        if message.len() < SC_BUF_SIZE {
            return hash_short(message, seed1, seed2);
        }
        
        let mut h = [
            seed1, seed2, SC_CONST,
            seed1, seed2, SC_CONST,
            seed1, seed2, SC_CONST,
            seed1, seed2, SC_CONST,
        ];
        
        // Process whole blocks
        let num_blocks = message.len() / SC_BLOCK_SIZE;
        for i in 0..num_blocks {
            let block = read_block_from_bytes(&message[i * SC_BLOCK_SIZE..]);
            mix(&block, &mut h);
        }
        
        // Handle last partial block
        let processed = num_blocks * SC_BLOCK_SIZE;
        let remainder = message.len() - processed;
        
        let mut buf = [0u64; SC_NUM_VARS];
        let buf_bytes = unsafe {
            core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, SC_BLOCK_SIZE)
        };
        buf_bytes[..remainder].copy_from_slice(&message[processed..]);
        buf_bytes[SC_BLOCK_SIZE - 1] = remainder as u8;
        
        end(&buf, &mut h);
        
        (h[0], h[1])
    }
    
    /// Hash a message in one call, returning 64-bit hash
    pub fn hash64(message: &[u8], seed: u64) -> u64 {
        let (h1, _) = Self::hash128(message, seed, seed);
        h1
    }
    
    /// Hash a message in one call, returning 32-bit hash
    pub fn hash32(message: &[u8], seed: u32) -> u32 {
        let (h1, _) = Self::hash128(message, seed as u64, seed as u64);
        h1 as u32
    }
}

/// Read a block of 12 u64s from a slice
#[inline(always)]
fn read_block(data: &[u64]) -> [u64; SC_NUM_VARS] {
    let mut block = [0u64; SC_NUM_VARS];
    block.copy_from_slice(&data[..SC_NUM_VARS]);
    block
}

/// Read a block of 12 u64s from bytes (little-endian)
#[inline(always)]
fn read_block_from_bytes(bytes: &[u8]) -> [u64; SC_NUM_VARS] {
    let mut block = [0u64; SC_NUM_VARS];
    for i in 0..SC_NUM_VARS {
        block[i] = read_u64_le(&bytes[i * 8..]);
    }
    block
}

impl Default for SpookyHash {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty() {
        let (h1, h2) = SpookyHash::hash128(b"", 0, 0);
        // Known values for empty string with seed (0, 0)
        assert_ne!(h1, 0);
        assert_ne!(h2, 0);
    }
    
    #[test]
    fn test_incremental() {
        let data = b"hello world this is a test message";
        
        // One-shot
        let (h1, h2) = SpookyHash::hash128(data, 0, 0);
        
        // Incremental
        let mut hasher = SpookyHash::new(0, 0);
        hasher.update(&data[..5]);
        hasher.update(&data[5..]);
        let (h1_inc, h2_inc) = hasher.finalize();
        
        assert_eq!(h1, h1_inc);
        assert_eq!(h2, h2_inc);
    }
    
    #[test]
    fn test_short_message() {
        // Test various short message lengths
        for len in 0..192 {
            let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let (h1, h2) = SpookyHash::hash128(&data, 0, 0);
            assert_ne!((h1, h2), (0, 0), "len={}", len);
        }
    }
}
