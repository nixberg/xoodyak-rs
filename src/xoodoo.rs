use std::simd::{simd_swizzle, u32x4, u8x16, ToBytes};

#[derive(Clone)]
pub struct Xoodoo(pub [u8; 48]);

impl Xoodoo {
    pub const fn new() -> Self {
        Self([0; 48])
    }

    pub fn permute(&mut self) {
        let mut a = u32x4::from_le_bytes(u8x16::from_array(self.0[00..16].try_into().unwrap()));
        let mut b = u32x4::from_le_bytes(u8x16::from_array(self.0[16..32].try_into().unwrap()));
        let mut c = u32x4::from_le_bytes(u8x16::from_array(self.0[32..48].try_into().unwrap()));

        for round_constant in [
            0x058, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ] {
            let p = (a ^ b ^ c).rotate_elements_right::<1>();
            let e = rotate_left::<5>(p) ^ rotate_left::<14>(p);
            a ^= e;
            b ^= e;
            c ^= e;

            b = b.rotate_elements_right::<1>();
            c = rotate_left::<11>(c);

            a ^= u32x4::from_array([round_constant, 0, 0, 0]);
            
            a ^= !b & c;
            b ^= !c & a;
            c ^= !a & b;

            b = rotate_left::<1>(b);
            c = u32x4::from_le_bytes(simd_swizzle!(
                c.to_le_bytes(), [11, 8, 9, 10, 15, 12, 13, 14, 3, 0, 1, 2, 7, 4, 5, 6]
            ))
        }

        self.0[00..16].copy_from_slice(a.to_le_bytes().as_array());
        self.0[16..32].copy_from_slice(b.to_le_bytes().as_array());
        self.0[32..48].copy_from_slice(c.to_le_bytes().as_array());
    }
}

#[inline(always)]
fn rotate_left<const OFFSET: u32>(x: u32x4) -> u32x4 {
    x << u32x4::splat(OFFSET) | x >> u32x4::splat(32 - OFFSET)
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
            xoodoo.0,
            [
                0xb0, 0xfa, 0x04, 0xfe, 0xce, 0xd8, 0xd5, 0x42, 0xe7, 0x2e, 0xc6, 0x29, 0xcf, 0xe5,
                0x7a, 0x2a, 0xa3, 0xeb, 0x36, 0xea, 0x0a, 0x9e, 0x64, 0x14, 0x1b, 0x52, 0x12, 0xfe,
                0x69, 0xff, 0x2e, 0xfe, 0xa5, 0x6c, 0x82, 0xf1, 0xe0, 0x41, 0x4c, 0xfc, 0x4f, 0x39,
                0x97, 0x15, 0xaf, 0x2f, 0x09, 0xeb,
            ]
        );
    }
}
