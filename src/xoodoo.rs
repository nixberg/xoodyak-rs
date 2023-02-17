#[derive(Clone)]
pub struct Xoodoo {
    bytes: [u8; 48],
}

impl Xoodoo {
    pub fn new() -> Xoodoo {
        Xoodoo { bytes: [0; 48] }
    }

    pub fn permute(&mut self) {
        let mut words = [0u32; 12];

        self.unpack(&mut words);

        let round_constants = &[
            0x058, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ];

        for &round_constant in round_constants {
            let mut e = [0u32; 4];

            for (i, e) in e.iter_mut().enumerate() {
                let p = words[i] ^ words[i + 4] ^ words[i + 8];
                *e = p.rotate_left(5) ^ p.rotate_left(14);
            }

            for (i, word) in words.iter_mut().enumerate() {
                *word ^= e[i.wrapping_sub(1) % 4];
            }

            words.swap(7, 4);
            words.swap(7, 5);
            words.swap(7, 6);

            words[0] ^= round_constant;

            for i in 0..4 {
                let a = words[i + 0];
                let b = words[i + 4];
                let c = words[i + 8].rotate_left(11);

                words[i + 8] = ((b & !a) ^ c).rotate_left(8);
                words[i + 4] = ((a & !c) ^ b).rotate_left(1);
                words[i] ^= c & !b;
            }

            words.swap(8, 10);
            words.swap(9, 11);
        }

        self.pack(&words);
    }

    #[inline(always)]
    pub fn bytes_view(&self) -> &[u8] {
        &self.bytes
    }

    #[inline(always)]
    pub fn bytes_view_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    #[inline]
    fn unpack(&self, destination: &mut [u32; 12]) {
        for (word, bytes) in destination.iter_mut().zip(self.bytes.chunks_exact(4)) {
            *word = u32::from_le_bytes(bytes.try_into().unwrap());
        }
    }

    #[inline]
    fn pack(&mut self, source: &[u32; 12]) {
        for (bytes, word) in self.bytes.chunks_exact_mut(4).zip(source.iter()) {
            bytes.copy_from_slice(&word.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Xoodoo;

    #[test]
    fn it_works() {
        let mut xoodoo = Xoodoo::new();

        for _ in 0..384 {
            xoodoo.permute();
        }

        assert_eq!(
            xoodoo.bytes,
            [
                0xb0, 0xfa, 0x04, 0xfe, 0xce, 0xd8, 0xd5, 0x42, 0xe7, 0x2e, 0xc6, 0x29, 0xcf, 0xe5,
                0x7a, 0x2a, 0xa3, 0xeb, 0x36, 0xea, 0x0a, 0x9e, 0x64, 0x14, 0x1b, 0x52, 0x12, 0xfe,
                0x69, 0xff, 0x2e, 0xfe, 0xa5, 0x6c, 0x82, 0xf1, 0xe0, 0x41, 0x4c, 0xfc, 0x4f, 0x39,
                0x97, 0x15, 0xaf, 0x2f, 0x09, 0xeb,
            ]
        );
    }
}
