use crate::error::{Error, ErrorKind, Result};
use crate::BitField;

pub struct MutableBitField<'a> {
    data: &'a mut [u8],
}

impl<'a> MutableBitField<'a> {
    pub fn new(data: &'a mut [u8]) -> MutableBitField<'a> {
        MutableBitField { data }
    }

    pub fn to_bitfield(self) -> BitField<'a> {
        BitField::new(self.data)
    }

    pub fn set_u8(&mut self, value: u8, start: usize, end: usize) -> Result<()> {
        if end >= start {
            let end_offset = end - start;
            if end_offset > 7 {
                Err(Error::with_context(
                    ErrorKind::InvParam,
                    &format!(
                        "get_unsigned_byte: too many bits {} to {} = {} > 8",
                        start,
                        end,
                        end_offset + 1
                    ),
                ))
            } else {
                let start_byte = start / 8;
                let start_bit = start % 8;
                if start_bit + end_offset < 8 {
                    match self.set_bits(value, start_byte, start_bit, start_bit + end_offset) {
                        Ok(_) => Ok(()),
                        Err(why) => Err(Error::with_all(
                            why.kind(),
                            &format!(
                                "set_unsigned_byte: error from set bits for bits {}:{} of {}",
                                start,
                                end,
                                self.data.len() * 8
                            ),
                            Box::new(why),
                        )),
                    }
                } else {
                    let last_offset = end_offset + start_bit - 8;
                    match self.set_bits(value >> (7 - start_bit) as u8, start_byte, start_bit, 7) {
                        Ok(_) => {
                            // println!("get_unsigned_byte: upper part: byte: {} start: {}, end: 7, res: {:08b}", start_byte , start , byte);
                            ()
                        }
                        Err(why) => {
                            return Err(Error::with_all(
                                why.kind(),
                                &format!(
                                    "get_unsigned_byte: error from get bits for bits {}:{} of {}",
                                    start,
                                    end,
                                    self.data.len() * 8
                                ),
                                Box::new(why),
                            ))
                        }
                    };
                    match self.set_bits(value, start_byte + 1, 0, last_offset) {
                        Ok(_) => {
                            // println!("get_unsigned_byte: lower part: byte: {} start: 0, end: {}, res: {:08b}", start_byte + 1, last_offset , byte);
                            Ok(())
                        }
                        Err(why) => {
                            return Err(Error::with_all(
                                why.kind(),
                                &format!(
                                    "get_unsigned_byte: error from get bits for bits {}:{} of {}",
                                    start,
                                    end,
                                    self.data.len() * 8
                                ),
                                Box::new(why),
                            ))
                        }
                    }
                }
            }
        } else {
            Err(Error::with_context(
                ErrorKind::OutOfRange,
                &format!("Start is greater that end {} > {}", start, end),
            ))
        }
    }

    // No checks done on this, ranges must be checked upstream
    fn create_mask(first: usize, last: usize) -> u8 {
        let mut byte: u8 = 0;
        for _ in first..=last {
            byte = (byte << 1) | 1;
        }
        if last < 7 {
            byte << (7 - last) as u8
        } else {
            byte
        }
    }

    fn set_bits(
        &mut self,
        value: u8,
        byte_offset: usize,
        start_bit: usize,
        end_bit: usize,
    ) -> Result<()> {
        if byte_offset < self.data.len() {
            let byte = &mut self.data[byte_offset];
            if start_bit < 8 && end_bit < 8 && start_bit <= end_bit {
                let mask = MutableBitField::create_mask(start_bit, end_bit);
                let or_value = if end_bit < 7 {
                    (value << (7 - end_bit) as u8) & mask
                } else {
                    value & mask
                };
                *byte = (*byte & !mask) | or_value;

                Ok(())
            } else {
                Err(Error::with_context(
                    ErrorKind::InvRange,
                    &format!(
                        "set_bits: bit index is invalid: {}..{} for byte",
                        start_bit, end_bit
                    ),
                ))
            }
        } else {
            Err(Error::with_context(
                ErrorKind::OutOfRange,
                &format!(
                    "set_bits: byte index is out of range: {} >= {}",
                    byte_offset,
                    self.data.len()
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mask() {
        assert_eq!(MutableBitField::create_mask(0, 0), 0b10000000);
        assert_eq!(MutableBitField::create_mask(4, 4), 0b00001000);
        assert_eq!(MutableBitField::create_mask(7, 7), 0b00000001);
        assert_eq!(MutableBitField::create_mask(0, 4), 0b11111000);
        assert_eq!(MutableBitField::create_mask(1, 4), 0b01111000);
        assert_eq!(MutableBitField::create_mask(2, 4), 0b00111000);
        assert_eq!(MutableBitField::create_mask(0, 7), 0b11111111);
    }

    #[test]
    fn test_set_bits() {
        let mut bytes: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let mut bitfield = MutableBitField::new(&mut bytes);
        bitfield.set_bits(0b00001111, 0, 4, 7).unwrap();
        assert_eq!(bytes[0], 0b10101111);
        let mut bytes: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let mut bitfield = MutableBitField::new(&mut bytes);
        bitfield.set_bits(0b00001111, 1, 0, 3).unwrap();
        assert_eq!(bytes[1], 0b11110101);
        let mut bytes: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let mut bitfield = MutableBitField::new(&mut bytes);
        bitfield.set_bits(0b00001111, 1, 2, 4).unwrap();
        assert_eq!(bytes[1], 0b01111101);
    }

    #[test]
    fn test_set_u8() {
        let mut bytes: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let mut bitfield = MutableBitField::new(&mut bytes);
        bitfield.set_u8(0b00001111, 4, 7).unwrap();
        assert_eq!(bytes[0], 0b10101111);
        assert_eq!(bytes[1], 0b01010101);

        let mut bytes: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let mut bitfield = MutableBitField::new(&mut bytes);
        bitfield.set_u8(0b00001111, 6, 9).unwrap();
        assert_eq!(bytes[0], 0b10101011);
        assert_eq!(bytes[1], 0b11010101);

        let mut bytes: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let mut bitfield = MutableBitField::new(&mut bytes);
        bitfield.set_u8(0b00001111, 6, 9).unwrap();
        assert_eq!(bytes[0], 0b10101011);
        assert_eq!(bytes[1], 0b11010101);
    }
}
