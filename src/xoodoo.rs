use std::mem::transmute;
use std::ops::{Index, IndexMut};

pub struct Xoodoo {
    state: [u32; 12],
}

impl Index<usize> for Xoodoo {
    type Output = u8;
    fn index<'a>(&'a self, i: usize) -> &'a u8 {
        let i = if cfg!(target_endian = "little") {
            i
        } else {
            i + 3 - 2 * (i % 4)
        };
        let bytes: &'a [u8; 48] = unsafe { transmute(&self.state) };
        &bytes[i]
    }
}

impl IndexMut<usize> for Xoodoo {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut u8 {
        let i = if cfg!(target_endian = "little") {
            i
        } else {
            i + 3 - 2 * (i % 4)
        };
        let bytes: &'a mut [u8; 48] = unsafe { transmute(&mut self.state) };
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

        let round_constants = [
            0x058, 0x038, 0x3c0, 0x0d0, 0x120, 0x014, 0x060, 0x02c, 0x380, 0x0f0, 0x1a0, 0x012,
        ];

        for round in 0..12 {
            let mut e = [0u32; 4];

            for i in 0..4 {
                e[i] = rotate(self.state[i] ^ self.state[i + 4] ^ self.state[i + 8], 18);
                e[i] ^= rotate(e[i], 9);
            }

            for i in 0..12 {
                self.state[i] ^= e[i.wrapping_sub(1) & 3];
            }

            self.state.swap(7, 4);
            self.state.swap(7, 5);
            self.state.swap(7, 6);
            self.state[0] ^= round_constants[round];

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
