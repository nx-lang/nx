//! The 64-bit FNV-1a hash NX writes into its artifacts.

/// A 64-bit FNV-1a hasher.
///
/// <para>Spelled out rather than taken from `DefaultHasher`, whose algorithm the standard library
/// does not promise across releases. What it hashes travels in NX IR — a module's fingerprint, a
/// standard library's version — so the same bytes must hash the same whatever toolchain, platform
/// or binding computed them.</para>
#[derive(Debug, Clone, Copy)]
pub struct Fnv1a64 {
    hash: u64,
}

impl Fnv1a64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub fn new() -> Self {
        Self {
            hash: Self::OFFSET_BASIS,
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(Self::PRIME);
        }
    }

    pub fn finish(&self) -> u64 {
        self.hash
    }
}

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Fnv1a64;

    #[test]
    fn matches_the_published_test_vectors() {
        let hash = |text: &str| {
            let mut hasher = Fnv1a64::new();
            hasher.write(text.as_bytes());
            hasher.finish()
        };
        assert_eq!(hash(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hash("foobar"), 0x8594_4171_f739_67e8);
    }
}
