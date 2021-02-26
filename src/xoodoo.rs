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

        for round_constant in &[
            0x058, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ] {
            let p = rotate(a ^ b ^ c);
            let e = rotate_lanes(p, 5) ^ rotate_lanes(p, 14);
            a ^= e;
            b ^= e;
            c ^= e;

            b = rotate(b);
            c = rotate_lanes(c, 11);

            a ^= u32x4::new(*round_constant, 0, 0, 0);

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
        let a_le: u32x4 = u8x16::from_slice_unaligned(&self.bytes[00..16]).into_bits();
        let b_le: u32x4 = u8x16::from_slice_unaligned(&self.bytes[16..32]).into_bits();
        let c_le: u32x4 = u8x16::from_slice_unaligned(&self.bytes[32..48]).into_bits();
        (
            u32x4::from_le(a_le),
            u32x4::from_le(b_le),
            u32x4::from_le(c_le),
        )
    }

    #[inline]
    fn pack(&mut self, a: u32x4, b: u32x4, c: u32x4) {
        let a_bytes: u8x16 = u32x4::to_le(a).into_bits();
        let b_bytes: u8x16 = u32x4::to_le(b).into_bits();
        let c_bytes: u8x16 = u32x4::to_le(c).into_bits();
        a_bytes.write_to_slice_unaligned(&mut self.bytes[00..16]);
        b_bytes.write_to_slice_unaligned(&mut self.bytes[16..32]);
        c_bytes.write_to_slice_unaligned(&mut self.bytes[32..48]);
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
