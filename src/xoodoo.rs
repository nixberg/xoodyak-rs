use std::simd::{simd_swizzle, u32x4, u8x16};

#[derive(Clone)]
pub struct Xoodoo {
    a: u32x4,
    b: u32x4,
    c: u32x4,
}

impl Xoodoo {
    pub fn new() -> Self {
        Self {
            a: u32x4::splat(0),
            b: u32x4::splat(0),
            c: u32x4::splat(0),
        }
    }

    pub fn permute(&mut self) {
        self.a.from_le();
        self.b.from_le();
        self.c.from_le();

        for round_constant in [
            0x058, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ] {
            let p = (self.a ^ self.b ^ self.c).rotate_lanes_right::<1>();
            let e = p.rotate_left::<5>() ^ p.rotate_left::<14>();
            self.a ^= e;
            self.b ^= e;
            self.c ^= e;

            self.b = self.b.rotate_lanes_right::<1>();
            self.c = self.c.rotate_left::<11>();

            self.a ^= u32x4::from_array([round_constant, 0, 0, 0]);

            self.a ^= !self.b & self.c;
            self.b ^= !self.c & self.a;
            self.c ^= !self.a & self.b;

            self.b = self.b.rotate_left::<1>();
            self.c = self.c.rho_east_part_two();
        }

        self.a.to_le();
        self.b.to_le();
        self.c.to_le();
    }

    #[inline(always)]
    pub fn bytes_view(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 48) }
    }

    #[inline(always)]
    pub fn bytes_view_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, 48) }
    }
}

trait XoodooInternal {
    fn from_le(&mut self);

    fn rotate_left<const OFFSET: u32>(&self) -> Self;

    fn rho_east_part_two(&self) -> Self;

    fn to_le(&mut self);
}

impl XoodooInternal for u32x4 {
    #[inline(always)]
    fn from_le(&mut self) {
        self.as_mut_array()
            .iter_mut()
            .for_each(|lane| *lane = u32::from_le(*lane));
    }

    #[inline(always)]
    fn rotate_left<const OFFSET: u32>(&self) -> Self {
        (self << Self::splat(OFFSET)) | (self >> Self::splat(32 - OFFSET))
    }

    #[inline(always)]
    fn rho_east_part_two(&self) -> Self {
        let mut bytes: u8x16 = unsafe { core::mem::transmute(*self) };
        bytes = simd_swizzle!(
            bytes,
            [11, 8, 9, 10, 15, 12, 13, 14, 3, 0, 1, 2, 7, 4, 5, 6]
        );
        unsafe { core::mem::transmute(bytes) }
    }

    #[inline(always)]
    fn to_le(&mut self) {
        self.as_mut_array()
            .iter_mut()
            .for_each(|lane| *lane = lane.to_le());
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
            xoodoo.bytes_view(),
            [
                0xb0, 0xfa, 0x04, 0xfe, 0xce, 0xd8, 0xd5, 0x42, 0xe7, 0x2e, 0xc6, 0x29, 0xcf, 0xe5,
                0x7a, 0x2a, 0xa3, 0xeb, 0x36, 0xea, 0x0a, 0x9e, 0x64, 0x14, 0x1b, 0x52, 0x12, 0xfe,
                0x69, 0xff, 0x2e, 0xfe, 0xa5, 0x6c, 0x82, 0xf1, 0xe0, 0x41, 0x4c, 0xfc, 0x4f, 0x39,
                0x97, 0x15, 0xaf, 0x2f, 0x09, 0xeb,
            ]
        );
    }
}
