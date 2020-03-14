use std::convert::TryInto;

#[derive(Clone)]
pub struct Xoodoo {
    pub state: [u8; 48],
}

impl Xoodoo {
    pub fn new() -> Xoodoo {
        Xoodoo { state: [0; 48] }
    }

    pub fn permute(&mut self) {
        let mut state = [0u32; 12];

        unpack(&self.state, &mut state);

        round(&mut state, 0x058);
        round(&mut state, 0x038);
        round(&mut state, 0x3c0);
        round(&mut state, 0x0d0);
        round(&mut state, 0x120);
        round(&mut state, 0x014);
        round(&mut state, 0x060);
        round(&mut state, 0x02c);
        round(&mut state, 0x380);
        round(&mut state, 0x0f0);
        round(&mut state, 0x1a0);
        round(&mut state, 0x012);

        pack(&state, &mut self.state);
    }
}

fn unpack(source: &[u8; 48], destination: &mut [u32; 12]) {
    for (word, bytes) in destination.iter_mut().zip(source.chunks_exact(4)) {
        *word = u32::from_le_bytes(bytes.try_into().unwrap());
    }
}

fn round(state: &mut [u32; 12], round_constant: u32) {
    #[inline(always)]
    fn rotate(v: u32, n: usize) -> u32 {
        (v >> n) | (v << (32 - n))
    }

    let mut e = [0u32; 4];

    for (i, e) in e.iter_mut().enumerate() {
        *e = rotate(state[i] ^ state[i + 4] ^ state[i + 8], 18);
        *e ^= rotate(*e, 9);
    }

    for (i, word) in state.iter_mut().enumerate() {
        *word ^= e[i.wrapping_sub(1) & 3];
    }

    state.swap(7, 4);
    state.swap(7, 5);
    state.swap(7, 6);
    state[0] ^= round_constant;

    for i in 0..4 {
        let a = state[i];
        let b = state[i + 4];
        let c = rotate(state[i + 8], 21);

        state[i + 8] = rotate((b & !a) ^ c, 24);
        state[i + 4] = rotate((a & !c) ^ b, 31);
        state[i] ^= c & !b;
    }

    state.swap(8, 10);
    state.swap(9, 11);
}

fn pack(source: &[u32; 12], destination: &mut [u8; 48]) {
    for (bytes, word) in destination.chunks_exact_mut(4).zip(source.iter()) {
        bytes.copy_from_slice(&word.to_le_bytes());
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
            0xb0, 0xfa, 0x04, 0xfe, 0xce, 0xd8, 0xd5, 0x42, 0xe7, 0x2e, 0xc6, 0x29, 0xcf, 0xe5,
            0x7a, 0x2a, 0xa3, 0xeb, 0x36, 0xea, 0x0a, 0x9e, 0x64, 0x14, 0x1b, 0x52, 0x12, 0xfe,
            0x69, 0xff, 0x2e, 0xfe, 0xa5, 0x6c, 0x82, 0xf1, 0xe0, 0x41, 0x4c, 0xfc, 0x4f, 0x39,
            0x97, 0x15, 0xaf, 0x2f, 0x09, 0xeb,
        ];

        for (s, e) in xoodoo.state.iter().zip(expected.iter()) {
            assert_eq!(s, e);
        }
    }
}
