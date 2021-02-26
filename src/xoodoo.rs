use packed_simd_2::{shuffle, u32x4, u8x16, IntoBits};

#[derive(Clone)]
pub struct Xoodoo {
    pub bytes: [u8; 48],
}

impl Xoodoo {
    pub fn new() -> Xoodoo {
        Xoodoo { bytes: [0; 48] }
    }

    pub fn permute(&mut self) {
        let (mut a, mut b, mut c) = self.unpack();

        let round_constants = &[
            0x058u32, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ];

        for &round_constant in round_constants {
            let p: u32x4 = rotate(a ^ b ^ c);
            let e: u32x4 = rotate_lanes(p, 5) ^ rotate_lanes(p, 14);
            a ^= e;
            b ^= e;
            c ^= e;

            b = rotate(b);
            c = rotate_lanes(c, 11);

            a ^= u32x4::new(round_constant, 0, 0, 0);

            a ^= !b & c;
            b ^= !c & a;
            c ^= !a & b;

            b = rotate_lanes(b, 1);
            c = rho_east_part_two(c);
        }

        self.pack(a, b, c);
    }

    #[inline]
    fn unpack(&self) -> (u32x4, u32x4, u32x4) {
        #[inline]
        fn read_from_slice(slice: &[u8]) -> u32x4 {
            let words_le: u32x4 = u8x16::from_slice_unaligned(slice).into_bits();
            u32x4::from_le(words_le)
        }
        let a = read_from_slice(&self.bytes[0..16]);
        let b = read_from_slice(&self.bytes[16..32]);
        let c = read_from_slice(&self.bytes[32..48]);
        (a, b, c)
    }

    #[inline]
    fn pack(&mut self, a: u32x4, b: u32x4, c: u32x4) {
        #[inline]
        fn write_to_slice(x: u32x4, slice: &mut [u8]) {
            let bytes_le: u8x16 = u32x4::to_le(x).into_bits();
            bytes_le.write_to_slice_unaligned(slice);
        }
        write_to_slice(a, &mut self.bytes[0..16]);
        write_to_slice(b, &mut self.bytes[16..32]);
        write_to_slice(c, &mut self.bytes[32..48]);
    }
}

#[inline]
fn rotate(x: u32x4) -> u32x4 {
    shuffle!(x, [3, 0, 1, 2])
}

#[inline]
fn rotate_lanes(x: u32x4, n: u32) -> u32x4 {
    x.rotate_left(u32x4::splat(n))
}

#[inline]
fn rho_east_part_two(c: u32x4) -> u32x4 {
    let mut bytes: u8x16 = c.into_bits();
    bytes = shuffle!(
        bytes,
        [11, 8, 9, 10, 15, 12, 13, 14, 3, 0, 1, 2, 7, 4, 5, 6]
    );
    bytes.into_bits()
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
