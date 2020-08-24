use crate::error::{Error, ErrorKind, Result, ToError};
use log::debug;

pub struct BitField<'a> {
    data: &'a [u8],
}

impl<'a> BitField<'a> {
    pub fn new(data: &'a [u8]) -> BitField<'a> {
        BitField { data }
    }

    /// Get a single bit
    pub fn get_bit(&self, index: usize) -> Result<bool> {
        Ok(self
            .get_bits(index / 8, index % 8, index % 8)
            .error_with_all(
                ErrorKind::OutOfRange,
                &format!(
                    "Bit index is out of range: {} >= {}",
                    index,
                    self.data.len() * 8,
                ),
            )?
            == 1)
    }

    /// Get a i32 value from the given offset and size
    pub fn get_i32_be(&self, start: usize, end: usize) -> Result<i32> {
        match self.get_u32_be(start, end) {
            Ok(byte) => Ok(BitField::twos_complement_u32(byte, 31 - (end - start))?),
            Err(why) => Err(Error::with_all(
                why.kind(),
                &format!("get_signed_byte: failure from get_unsigned_u16"),
                Box::new(why),
            )),
        }
    }

    /// Get a u32 value from the given offset and size
    pub fn get_u32_be(&self, start: usize, end: usize) -> Result<u32> {
        let bit_offset = end - start;
        if bit_offset > 15 {
            let (byte_0, mut next) = if bit_offset > 23 {
                let median = start + (bit_offset - 24);
                (self.get_u8(start, median)?, median + 1)
            } else {
                (0, start)
            };
            let byte_1 = self.get_u8(next, next + 7)?;
            next += 8;
            let byte_2 = self.get_u8(next, next + 7)?;
            next += 8;
            let byte_3 = self.get_u8(next, next + 7)?;
            Ok(((byte_0 as u32) << 24)
                | ((byte_1 as u32) << 16)
                | ((byte_2 as u32) << 8)
                | byte_3 as u32)
        } else if bit_offset > 7 {
            Ok(self.get_u16_be(start, end)? as u32)
        } else {
            Ok(self.get_u8(start, end)? as u32)
        }
    }

    /// Get a i16 value from the given offset and size
    pub fn get_i16_be(&self, start: usize, end: usize) -> Result<i16> {
        match self.get_u16_be(start, end) {
            Ok(byte) => Ok(BitField::twos_complement_u16(byte, 15 - (end - start))?),
            Err(why) => Err(Error::with_all(
                why.kind(),
                &format!("get_signed_byte: failure from get_unsigned_u16"),
                Box::new(why),
            )),
        }
    }

    /// Get a u16 value from the given offset and size
    pub fn get_u16_be(&self, start: usize, end: usize) -> Result<u16> {
        let bit_offset = end - start;
        if bit_offset > 7 {
            let median = start + (bit_offset - 8);
            let lower = self.get_u8(start, median)?;
            let upper = self.get_u8(median + 1, end)?;
            Ok(((lower as u16) << 8) | upper as u16)
        } else {
            Ok(self.get_u8(start, end)? as u16)
        }
    }

    /// Get a ui8 value from the given offset and size
    pub fn get_i8(&self, start: usize, end: usize) -> Result<i8> {
        match self.get_u8(start, end) {
            Ok(byte) => Ok(BitField::twos_complement_u8(byte, 7 - (end - start))?),
            Err(why) => Err(Error::with_all(
                why.kind(),
                &format!("get_signed_byte: failure from get_unsigned_u8"),
                Box::new(why),
            )),
        }
    }
    /// Get a u8 value from the given offset and size
    pub fn get_u8(&self, start: usize, end: usize) -> Result<u8> {
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
                    match self.get_bits(start_byte, start_bit, start_bit + end_offset) {
                        Ok(byte) => Ok(byte),
                        Err(why) => Err(Error::with_all(
                            why.kind(),
                            &format!(
                                "get_unsigned_byte: error from get bits for bits {}:{} of {}",
                                start,
                                end,
                                self.data.len() * 8
                            ),
                            Box::new(why),
                        )),
                    }
                } else {
                    let last_offset = end_offset + start_bit - 8;
                    Ok((match self.get_bits(start_byte, start_bit, 7) {
                        Ok(byte) => {
                            // println!("get_unsigned_byte: upper part: byte: {} start: {}, end: 7, res: {:08b}", start_byte , start , byte);
                            byte
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
                    } << last_offset as u8 + 1)
                        | match self.get_bits(start_byte + 1, 0, last_offset) {
                            Ok(byte) => {
                                // println!("get_unsigned_byte: lower part: byte: {} start: 0, end: {}, res: {:08b}", start_byte + 1, last_offset , byte);
                                byte
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
                        })
                }
            }
        } else {
            Err(Error::with_context(
                ErrorKind::OutOfRange,
                &format!("Start is greater that end {} > {}", start, end),
            ))
        }
    }

    fn get_bits(&self, byte_offset: usize, start_bit: usize, end_bit: usize) -> Result<u8> {
        if byte_offset < self.data.len() {
            let mut byte = self.data[byte_offset];
            if start_bit < 8 && end_bit < 8 {
                if start_bit <= end_bit {
                    if start_bit > 0 {
                        byte = byte << start_bit as u8;
                    }

                    let right_shift = 7 - end_bit + start_bit;
                    if right_shift > 0 {
                        byte = byte >> right_shift as u8;
                    }
                    Ok(byte)
                } else {
                    Err(Error::with_context(
                        ErrorKind::InvParam,
                        &format!("get_bits: start_bit > end_bit: {} > {}", start_bit, end_bit),
                    ))
                }
            } else {
                Err(Error::with_context(
                    ErrorKind::OutOfRange,
                    &format!(
                        "get_bits: bit index is out of range: {} or {} >= 8",
                        start_bit, end_bit
                    ),
                ))
            }
        } else {
            Err(Error::with_context(
                ErrorKind::OutOfRange,
                &format!(
                    "get_bits: byte index is out of range: {} >= {}",
                    byte_offset,
                    self.data.len()
                ),
            ))
        }
    }

    fn twos_complement_u32(val: u32, sign_bit: usize) -> Result<i32> {
        debug!("twos_complement_u32: {:x}, {}", val, sign_bit);
        let mask = 1 << (31 - sign_bit);
        debug!("twos_complement_u32: mask {:x}", mask);
        if val as u64 & mask != 0 {
            Ok(-(((mask << 1) - val as u64) as i32))
        } else {
            Ok(val as i32)
        }
    }

    fn twos_complement_u16(val: u16, sign_bit: usize) -> Result<i16> {
        let mask = 1 << (15 - sign_bit);
        if val as u32 & mask != 0 {
            Ok(-(((mask << 1) - val as u32) as i16))
        } else {
            Ok(val as i16)
        }
    }

    fn twos_complement_u8(val: u8, sign_bit: usize) -> Result<i8> {
        let mask = 1 << (7 - sign_bit);
        if val as u16 & mask != 0 {
            Ok(-(((mask << 1) - val as u16) as i8))
        } else {
            Ok(val as i8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bits() {
        const BYTES: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
        let bitfield = BitField::new(&BYTES);

        assert_eq!(bitfield.get_bit(0).unwrap(), true);
        assert_eq!(bitfield.get_bit(1).unwrap(), false);
        assert_eq!(bitfield.get_bit(2).unwrap(), true);
        assert_eq!(bitfield.get_bit(3).unwrap(), false);
        assert_eq!(bitfield.get_bit(4).unwrap(), true);
        assert_eq!(bitfield.get_bit(5).unwrap(), false);
        assert_eq!(bitfield.get_bit(6).unwrap(), true);
        assert_eq!(bitfield.get_bit(7).unwrap(), false);

        assert_eq!(bitfield.get_bit(8).unwrap(), false);
        assert_eq!(bitfield.get_bit(9).unwrap(), true);
        assert_eq!(bitfield.get_bit(10).unwrap(), false);
        assert_eq!(bitfield.get_bit(11).unwrap(), true);
        assert_eq!(bitfield.get_bit(12).unwrap(), false);
        assert_eq!(bitfield.get_bit(13).unwrap(), true);
        assert_eq!(bitfield.get_bit(14).unwrap(), false);
        assert_eq!(bitfield.get_bit(15).unwrap(), true);

        assert_eq!(bitfield.get_bit(16).unwrap(), true);
        assert_eq!(bitfield.get_bit(17).unwrap(), false);
        assert_eq!(bitfield.get_bit(18).unwrap(), true);
        assert_eq!(bitfield.get_bit(19).unwrap(), false);
        assert_eq!(bitfield.get_bit(20).unwrap(), true);
        assert_eq!(bitfield.get_bit(21).unwrap(), false);
        assert_eq!(bitfield.get_bit(22).unwrap(), true);
        assert_eq!(bitfield.get_bit(23).unwrap(), false);
    }

    #[test]
    fn test_get_u8() {
        const BYTES: [u8; 3] = [0b10101010, 0b10101010, 0b10101010];
        let bitfield = BitField::new(&BYTES);
        // 8 - 1 bits right justified
        assert_eq!(bitfield.get_u8(0, 7).unwrap(), 0b10101010);
        assert_eq!(bitfield.get_u8(1, 7).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(2, 7).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(3, 7).unwrap(), 0b00001010);
        assert_eq!(bitfield.get_u8(4, 7).unwrap(), 0b00001010);
        assert_eq!(bitfield.get_u8(5, 7).unwrap(), 0b00000010);
        assert_eq!(bitfield.get_u8(6, 7).unwrap(), 0b00000010);
        assert_eq!(bitfield.get_u8(7, 7).unwrap(), 0b00000000);

        // 7 - 2 bits left justified
        assert_eq!(bitfield.get_u8(0, 6).unwrap(), 0b01010101);
        assert_eq!(bitfield.get_u8(0, 5).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(0, 4).unwrap(), 0b00010101);
        assert_eq!(bitfield.get_u8(0, 3).unwrap(), 0b00001010);
        assert_eq!(bitfield.get_u8(0, 2).unwrap(), 0b00000101);
        assert_eq!(bitfield.get_u8(0, 1).unwrap(), 0b00000010);

        // 6 bits crossing into byte 2 bits left justified
        assert_eq!(bitfield.get_u8(2, 8).unwrap(), 0b01010101);
        assert_eq!(bitfield.get_u8(3, 9).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(4, 10).unwrap(), 0b01010101);
        assert_eq!(bitfield.get_u8(5, 11).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(6, 12).unwrap(), 0b01010101);
        assert_eq!(bitfield.get_u8(7, 13).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(8, 14).unwrap(), 0b01010101);
        assert_eq!(bitfield.get_u8(9, 15).unwrap(), 0b00101010);

        // 5 bits crossing into byte 2 bits left justified
        assert_eq!(bitfield.get_u8(3, 8).unwrap(), 0b00010101);
        assert_eq!(bitfield.get_u8(4, 9).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(5, 10).unwrap(), 0b00010101);
        assert_eq!(bitfield.get_u8(6, 11).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(7, 12).unwrap(), 0b00010101);
        assert_eq!(bitfield.get_u8(8, 13).unwrap(), 0b00101010);
        assert_eq!(bitfield.get_u8(9, 14).unwrap(), 0b00010101);
        assert_eq!(bitfield.get_u8(10, 15).unwrap(), 0b00101010);
    }

    #[test]
    fn test_get_i8() {
        const BYTES: [u8; 3] = [0b00000000, 0b11111111, 0b10101010];
        let bitfield = BitField::new(&BYTES);
        assert_eq!(bitfield.get_i8(8, 15).unwrap(), -1);
        assert_eq!(bitfield.get_i8(9, 15).unwrap(), -1);
        assert_eq!(bitfield.get_i8(10, 15).unwrap(), -1);
        assert_eq!(bitfield.get_i8(11, 15).unwrap(), -1);
        assert_eq!(bitfield.get_i8(12, 15).unwrap(), -1);
        assert_eq!(bitfield.get_i8(13, 15).unwrap(), -1);
        assert_eq!(bitfield.get_i8(7, 14).unwrap(), 127);
        const BYTE1: u8 = 0b10101010;
        const BYTE2: u8 = 0b11101010;
        const BYTE3: u8 = 0b11110101;
        const BYTE4: u8 = 0b11111010;

        assert_eq!(bitfield.get_i8(16, 23).unwrap(), (BYTE1 as i8));
        assert_eq!(bitfield.get_i8(18, 23).unwrap(), (BYTE2 as i8));
        assert_eq!(bitfield.get_i8(14, 21).unwrap(), (BYTE2 as i8));
        assert_eq!(bitfield.get_i8(14, 20).unwrap(), (BYTE3 as i8));
        assert_eq!(bitfield.get_i8(14, 19).unwrap(), (BYTE4 as i8));
    }

    #[test]
    fn test_get_i32() {
        const BYTES: [u8; 6] = [
            0b00000000, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b10101010,
        ];
        let bitfield = BitField::new(&BYTES);
        assert_eq!(bitfield.get_i32_be(8, 39).unwrap(), -1);
        assert_eq!(bitfield.get_i32_be(9, 39).unwrap(), -1);
        assert_eq!(bitfield.get_i32_be(10, 39).unwrap(), -1);
        assert_eq!(bitfield.get_i32_be(11, 39).unwrap(), -1);
        assert_eq!(bitfield.get_i32_be(12, 39).unwrap(), -1);
        assert_eq!(bitfield.get_i32_be(13, 39).unwrap(), -1);
        assert_eq!(bitfield.get_i32_be(7, 38).unwrap(), 0x7FFFFFFF as i32);
        assert_eq!(bitfield.get_i32_be(16, 47).unwrap(), -86);
    }

    #[test]
    fn test_get_i16() {
        const BYTES: [u8; 4] = [0b00000000, 0b11111111, 0b11111111, 0b10101010];
        let bitfield = BitField::new(&BYTES);
        assert_eq!(bitfield.get_i16_be(8, 23).unwrap(), -1);
        assert_eq!(bitfield.get_i16_be(9, 23).unwrap(), -1);
        assert_eq!(bitfield.get_i16_be(10, 23).unwrap(), -1);
        assert_eq!(bitfield.get_i16_be(11, 23).unwrap(), -1);
        assert_eq!(bitfield.get_i16_be(12, 23).unwrap(), -1);
        assert_eq!(bitfield.get_i16_be(13, 23).unwrap(), -1);
        assert_eq!(bitfield.get_i16_be(7, 22).unwrap(), 0x7FFF as i16);
    }

    #[test]
    fn test_get_u16_be() {
        const BYTES: [u8; 3] = [0b10101010, 0b10101010, 0b10101010];
        let bitfield = BitField::new(&BYTES);
        assert_eq!(bitfield.get_u16_be(0, 15).unwrap(), 0b1010101010101010);
        assert_eq!(bitfield.get_u16_be(1, 16).unwrap(), 0b0101010101010101);
        assert_eq!(bitfield.get_u16_be(2, 17).unwrap(), 0b1010101010101010);
        assert_eq!(bitfield.get_u16_be(2, 16).unwrap(), 0b0101010101010101);
        assert_eq!(bitfield.get_u16_be(2, 15).unwrap(), 0b0010101010101010);
        assert_eq!(bitfield.get_u16_be(2, 14).unwrap(), 0b0001010101010101);
        assert_eq!(bitfield.get_u16_be(2, 13).unwrap(), 0b0000101010101010);
        assert_eq!(bitfield.get_u16_be(2, 12).unwrap(), 0b0000010101010101);
        assert_eq!(bitfield.get_u16_be(2, 11).unwrap(), 0b0000001010101010);
        assert_eq!(bitfield.get_u16_be(2, 10).unwrap(), 0b0000000101010101);
        assert_eq!(bitfield.get_u16_be(2, 9).unwrap(), 0b0000000010101010);
    }

    #[test]
    fn test_get_u32_be() {
        const BYTES: [u8; 6] = [
            0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010, 0b10101010,
        ];

        let bitfield = BitField::new(&BYTES);
        assert_eq!(
            bitfield.get_u32_be(0, 31).unwrap(),
            0b10101010101010101010101010101010
        );
        assert_eq!(
            bitfield.get_u32_be(1, 32).unwrap(),
            0b001010101010101010101010101010101
        );
        assert_eq!(
            bitfield.get_u32_be(2, 32).unwrap(),
            0b001010101010101010101010101010101
        );
        assert_eq!(
            bitfield.get_u32_be(3, 32).unwrap(),
            0b000010101010101010101010101010101
        );
        assert_eq!(
            bitfield.get_u32_be(4, 32).unwrap(),
            0b000010101010101010101010101010101
        );

        assert_eq!(
            bitfield.get_u32_be(5, 32).unwrap(),
            0b000000101010101010101010101010101
        );

        assert_eq!(
            bitfield.get_u32_be(6, 32).unwrap(),
            0b000000101010101010101010101010101
        );

        assert_eq!(
            bitfield.get_u32_be(7, 32).unwrap(),
            0b000000001010101010101010101010101
        );

        assert_eq!(
            bitfield.get_u32_be(8, 32).unwrap(),
            0b000000001010101010101010101010101
        );
    }
}
