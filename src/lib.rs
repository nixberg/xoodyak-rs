mod blocks;
mod xoodoo;

use blocks::Blockable;
use xoodoo::Xoodoo;

enum Flag {
    Zero,
    AbsorbKey,
    Absorb,
    Ratchet,
    SqueezeKey,
    Squeeze,
    Crypt,
}

impl Flag {
    fn value(&self) -> u8 {
        match self {
            Flag::Zero => 0x00,
            Flag::AbsorbKey => 0x02,
            Flag::Absorb => 0x03,
            Flag::Ratchet => 0x10,
            Flag::SqueezeKey => 0x20,
            Flag::Squeeze => 0x40,
            Flag::Crypt => 0x80,
        }
    }
}

#[derive(PartialEq)]
enum Mode {
    Hash,
    Keyed,
}

struct Rates {
    absorb: usize,
    squeeze: usize,
}

impl Rates {
    const HASH: usize = 16;
    const INPUT: usize = 44;
    const OUTPUT: usize = 24;
    const RATCHET: usize = 16;
}

#[derive(PartialEq)]
enum Phase {
    Up,
    Down,
}

pub struct Xoodyak {
    mode: Mode,
    rates: Rates,
    phase: Phase,
    xoodoo: Xoodoo,
}

impl Xoodyak {
    pub fn new() -> Xoodyak {
        Xoodyak {
            mode: Mode::Hash,
            rates: Rates {
                absorb: Rates::HASH,
                squeeze: Rates::HASH,
            },
            phase: Phase::Up,
            xoodoo: Xoodoo::new(),
        }
    }

    pub fn keyed(key: &[u8], id: &[u8], counter: &[u8]) -> Xoodyak {
        assert!(key.len() + id.len() <= Rates::INPUT);

        let mut xoodyak = Xoodyak {
            mode: Mode::Keyed,
            rates: Rates {
                absorb: Rates::INPUT,
                squeeze: Rates::OUTPUT,
            },
            phase: Phase::Up,
            xoodoo: Xoodoo::new(),
        };

        let bytes = [key, id, &[id.len() as u8]].concat();
        xoodyak.absorb_any(&bytes, xoodyak.rates.absorb, Flag::AbsorbKey);

        if counter.len() > 0 {
            xoodyak.absorb_any(counter, 1, Flag::Zero);
        }

        xoodyak
    }

    fn absorb_any(&mut self, data: &[u8], rate: usize, down_flag: Flag) {
        let mut down_flag = down_flag;
        for block in data.blocks(rate) {
            if self.phase != Phase::Up {
                self.up(Flag::Zero);
            }
            self.down(block, down_flag);
            down_flag = Flag::Zero;
        }
    }

    pub fn crypt(&mut self, input: &[u8], output: &mut [u8], decrypt: bool) {
        let mut flag = Flag::Crypt;
        let mut offset = 0;
        for block in input.blocks(Rates::OUTPUT) {
            self.up(flag);
            flag = Flag::Zero;
            for (i, byte) in block.iter().enumerate() {
                output[i] = byte ^ self.xoodoo[i];
            }
            if decrypt {
                self.down(&output[offset..(offset + block.len())], Flag::Zero);
                offset += block.len();
            } else {
                self.down(block, Flag::Zero);
            }
        }
    }

    fn squeeze_any_to(&mut self, buffer: &mut [u8], up_flag: Flag) {
        assert!(buffer.len() > 0);
        let mut chunks = buffer.chunks_mut(self.rates.squeeze);
        self.up_to(chunks.next().unwrap(), up_flag);
        for chunk in chunks {
            self.down(&[], Flag::Zero);
            self.up_to(chunk, Flag::Zero);
        }
    }

    fn down(&mut self, block: &[u8], flag: Flag) {
        self.phase = Phase::Down;
        for (i, byte) in block.iter().enumerate() {
            self.xoodoo[i] ^= byte;
        }
        self.xoodoo[block.len()] ^= 0x01;
        self.xoodoo[47] ^= if self.mode == Mode::Hash {
            flag.value() & 0x01
        } else {
            flag.value()
        };
    }

    fn up(&mut self, flag: Flag) {
        self.phase = Phase::Up;
        if self.mode != Mode::Hash {
            self.xoodoo[47] ^= flag.value();
        }
        self.xoodoo.permute();
    }

    fn up_to(&mut self, block: &mut [u8], flag: Flag) {
        self.up(flag);
        for (i, byte) in block.iter_mut().enumerate() {
            *byte = self.xoodoo[i];
        }
    }

    pub fn absorb(&mut self, data: &[u8]) {
        self.absorb_any(data, self.rates.absorb, Flag::Absorb);
    }

    pub fn encrypt(&mut self, plaintext: &[u8], ciphertext: &mut [u8]) {
        assert!(self.mode == Mode::Keyed);
        self.crypt(plaintext, ciphertext, false);
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], plaintext: &mut [u8]) {
        assert!(self.mode == Mode::Keyed);
        self.crypt(ciphertext, plaintext, true);
    }

    pub fn squeeze_to(&mut self, buffer: &mut [u8]) {
        self.squeeze_any_to(buffer, Flag::Squeeze);
    }

    pub fn squeeze_key_to(&mut self, buffer: &mut [u8]) {
        self.squeeze_any_to(buffer, Flag::SqueezeKey);
    }

    pub fn ratchet(&mut self) {
        assert!(self.mode == Mode::Keyed);
        let mut buffer = [0u8; Rates::RATCHET];
        self.squeeze_any_to(&mut buffer, Flag::Ratchet);
        self.absorb_any(&mut buffer, self.rates.absorb, Flag::Zero);
    }
}

#[cfg(test)]
mod tests {
    extern crate hex;

    use super::Xoodyak;

    #[test]
    fn hash_mode() {
        struct KAT<'a> {
            msg: &'a str,
            md: &'a str,
        }

        let kats = [
            KAT {
                msg: "",
                md: "EA152F2B47BCE24EFB66C479D4ADF17BD324D806E85FF75EE369EE50DC8F8BD1"
            },
            KAT {
                msg: "00",
                md: "27921F8DDF392894460B70B3ED6C091E6421B7D2147DCD6031D7EFEBAD3030CC"
            },
            KAT {
                msg: "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232425262728",
                md: "079BFF70855D0767CC3349752F3DEFF2B01D44A15EF68B98C9BCDF20BD1970D8"
            },
        ];

        for kat in kats.iter() {
            let msg_bytes = hex::decode(kat.msg).unwrap();
            let md_bytes = hex::decode(kat.md).unwrap();

            let mut xoodyak = Xoodyak::new();
            xoodyak.absorb(&msg_bytes);
            let mut new_md_bytes = vec![0; md_bytes.len()];
            xoodyak.squeeze_to(&mut new_md_bytes);

            assert_eq!(md_bytes, new_md_bytes);
        }
    }

    #[test]
    fn aead_mode() {
        struct KAT<'a> {
            key: &'a str,
            nonce: &'a str,
            pt: &'a str,
            ad: &'a str,
            ct: &'a str,
        }

        let kats = [
            KAT {
                key: "000102030405060708090A0B0C0D0E0F",
                nonce: "000102030405060708090A0B0C0D0E0F",
                pt: "",
                ad: "",
                ct: "4BF0E393144CB58069FC1FEBCAFCFB3C",
            },
            KAT {
                key: "000102030405060708090A0B0C0D0E0F",
                nonce: "000102030405060708090A0B0C0D0E0F",
                pt: "000102030405060708090A0B0C",
                ad: "000102030405060708090A0B0C0D0E0F101112131415",
                ct: "CFA1C6EFB6E4795450ABF50494C96372BF566DEC846DBAE29C36F4A9CF",
            },
        ];

        for kat in kats.iter() {
            let key = hex::decode(kat.key).unwrap();
            let nonce = hex::decode(kat.nonce).unwrap();
            let pt = hex::decode(kat.pt).unwrap();
            let ad = hex::decode(kat.ad).unwrap();
            let ct = hex::decode(kat.ct).unwrap();
            let (ct_only, tag) = ct.split_at(pt.len());

            let mut xoodyak = Xoodyak::keyed(&key, &[], &[]);
            xoodyak.absorb(&nonce);
            xoodyak.absorb(&ad);
            let mut new_ct = vec![0; ct.len()];
            let (new_ct_only, new_tag) = new_ct.split_at_mut(pt.len());
            xoodyak.encrypt(&pt, new_ct_only);
            xoodyak.squeeze_to(new_tag);

            assert_eq!(ct, new_ct);

            xoodyak = Xoodyak::keyed(&key, &[], &[]);
            xoodyak.absorb(&nonce);
            xoodyak.absorb(&ad);
            let mut new_pt = vec![0; pt.len()];
            xoodyak.decrypt(ct_only, &mut new_pt);
            let mut new_tag = vec![0; tag.len()];
            xoodyak.squeeze_to(&mut new_tag);

            assert_eq!(pt, new_pt);
            assert_eq!(tag, new_tag.as_slice());
        }
    }
}
