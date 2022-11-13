use std::{collections::{hash_map::RandomState, HashMap, HashSet}, time::Instant, cell::Cell, hash::{BuildHasher, Hash, Hasher}};

// easy way to "steal" entropy from the standard library. Fairly slow, so we merely use it to initialize a small thread-local RNG to seed our hashers with
fn generate_seed() -> u64 {
    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    Instant::now().hash(&mut hasher);
    hasher.finish()
}

// RNG from https://github.com/tkaitchuck/Mwc256XXA64
#[derive(Debug, Clone)]
struct Rng {
    state: Cell<[u64; 4]>
}
impl Rng {
    fn from_seed(s0: u64, s1: u64) -> Self { 
        let res = Self { state: Cell::new([s0, s1, 0xcafef00dd15ea5e5, 0x14057B7EF767814F]) };
        for _ in 0..6 { res.gen_64(); }
        res
    }
    fn new() -> Self { Self::from_seed(generate_seed(), generate_seed()) }
    fn gen_64(&self) -> u64 {
        let [x1, x2, x3, c] = self.state.get();
        let t = (x3 as u128).wrapping_mul(0xfeb3_4465_7c0a_f413);
        let (low, hi) = (t as u64, (t >> 64) as u64);
        let res = (x3 ^ x2).wrapping_add(x1 ^ hi);
        let (x0, b) = low.overflowing_add(c);
        self.state.set([x0, x1, x2, hi.wrapping_add(b as u64)]);
        res
    }
}

/// The SHash hasher. Implements both BuildHasher and Hasher.
#[derive(Debug, Clone, Copy)]
pub struct SHash(u64, u128);
impl SHash {
    pub fn new() -> Self {
        thread_local! {
            static RNG: Rng = Rng::new();
        }

        RNG.with(|rng| {
            Self::from_seed(rng.gen_64(), rng.gen_64(), rng.gen_64())
        })
    }
 
    pub fn from_seed(k: u64, m: u64, m2: u64) -> Self {
        Self(k, m as u128 | (m2 as u128) << 64 | 1)
    }
}
impl BuildHasher for SHash {
    type Hasher = Self;
    fn build_hasher(&self) -> Self::Hasher {
        *self
    }
}
impl Hasher for SHash {
    fn write(&mut self, mut bytes: &[u8]) {
        while bytes.len() >= 8 {
            let x = bytes.as_ptr() as *const u64;
            let x = unsafe { x.read_unaligned() };
            self.write_u64(x);
            bytes = &bytes[8..];
        }
        if bytes.len() > 0 {
            let mut x = [!0u8; 8];
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), x.as_mut_ptr(), bytes.len()) }
            let x = u64::from_ne_bytes(x);
            self.write_u64(x);
        }
    }
    fn write_u8(&mut self, i: u8) { self.write_u64(i as _) }
    fn write_u16(&mut self, i: u16) { self.write_u64(i as _) }
    fn write_u32(&mut self, i: u32) { self.write_u64(i as _) }
    fn write_usize(&mut self, i: usize) { self.write_u64(i as _) }
    #[inline] fn write_u64(&mut self, i: u64) {
        let mut z = i.wrapping_add((self.1 >> 64) as u64);
        z ^= z.rotate_right(25) ^ z.rotate_right(47);
        z = z.wrapping_mul(0x9E6C63D0676A9A99).wrapping_add(self.0);
        z ^= z >> 23 ^ z >> 51;
        z = z.wrapping_mul(0x9E6D62D06F6A9A9B);
        z ^= z >> 23 ^ z >> 51;
        self.0 = z;
        self.1 = self.1.wrapping_mul(0xda942042e4dd58b5);
    }
    fn write_u128(&mut self, i: u128) { self.write_u64(i as _); self.write_u64((i >> 64) as _); }
    fn finish(&self) -> u64 {
        self.0
    }
}
impl Default for SHash { fn default() -> Self { Self::new() } }

/// convenient type alias for SHash hashmaps. Note that you should use SHashMap::default() to easily instantiate an empty hashmap.
pub type SHashMap<K, V> = HashMap<K, V, SHash>;

/// convenient type alias for SHash hashsets. Note that you should use SHashSet::default() to easily instantiate an empty hashset.
pub type SHashSet<K> = HashSet<K, SHash>;


#[cfg(test)]
mod tests {
    use crate::SHashMap;

    #[test]
    fn simple_hashmap_test() {
        let mut map = SHashMap::default();
        map.insert("Adventures of Huckleberry Finn".to_string(), "My favorite book.".to_string());
        assert_eq!(map.get("Adventures of Huckleberry Finn"), Some(&"My favorite book.".to_string()));
    }
}
