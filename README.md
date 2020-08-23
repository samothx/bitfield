### Bitfield implementation for parsing NMEA2k / NMEA 0186 Messages 

Create bitfield from [u8]: 

```rust
use bitfield::Bitfield;

const BYTES1: [u8; 3] = [0b10101010, 0b01010101, 0b10101010];
let bitfield = BitField::new(&BYTES1);
assert_eq!(bitfield.get_bit(3).unwrap(), false);


const BYTES2: [u8; 3] = [0b10101010, 0b10101010, 0b10101010];
let bitfield = BitField::new(&BYTES2);
assert_eq!(bitfield.get_u16_be(0, 15).unwrap(), 0b1010101010101010);
```
