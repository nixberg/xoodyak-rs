use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct Xoodoo {
    state: [u32; 12],
}

impl Index<usize> for Xoodoo {
    type Output = u8;
    fn index(&self, i: usize) -> &u8 {
        let bytes = unsafe { &*(&self.state as *const [u32; 12] as *const [u8; 48]) };
        &bytes[i]
    }
}

impl IndexMut<usize> for Xoodoo {
    fn index_mut(&mut self, i: usize) -> &mut u8 {
        let bytes = unsafe { &mut *(&mut self.state as *mut [u32; 12] as *mut [u8; 48]) };
        &mut bytes[i]
    }
}

impl Xoodoo {
    pub fn new() -> Xoodoo {
        Xoodoo { state: [0; 12] }
    }

    pub fn permute(&mut self) {
        #[inline(always)]
        fn rotate(v: u32, n: usize) -> u32 {
            (v >> n) | (v << (32 - n))
        }

        for i in 0..12 {
            self.state[i] = u32::from_le(self.state[i]);
        }

        let round_constants = [
            0x058, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ];

        for round_constant in round_constants.iter() {
            let mut e = [0u32; 4];

            for (i, e) in e.iter_mut().enumerate() {
                *e = rotate(self.state[i] ^ self.state[i + 4] ^ self.state[i + 8], 18);
                *e ^= rotate(*e, 9);
            }

            for i in 0..12 {
                self.state[i] ^= e[i.wrapping_sub(1) & 3];
            }

            self.state.swap(7, 4);
            self.state.swap(7, 5);
            self.state.swap(7, 6);
            self.state[0] ^= round_constant;

            for i in 0..4 {
                let a = self.state[i];
                let b = self.state[i + 4];
                let c = rotate(self.state[i + 8], 21);

                self.state[i + 8] = rotate((b & !a) ^ c, 24);
                self.state[i + 4] = rotate((a & !c) ^ b, 31);
                self.state[i] ^= c & !b;
            }

            self.state.swap(8, 10);
            self.state.swap(9, 11);
        }

        for i in 0..12 {
            self.state[i] = self.state[i].to_le();
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

        let expected = [
            0xfe04fab0, 0x42d5d8ce, 0x29c62ee7, 0x2a7ae5cf, 0xea36eba3, 0x14649e0a, 0xfe12521b,
            0xfe2eff69, 0xf1826ca5, 0xfc4c41e0, 0x1597394f, 0xeb092faf,
        ];

        for i in 0..12 {
            assert_eq!(xoodoo.state[i], expected[i]);
        }
    }
}
