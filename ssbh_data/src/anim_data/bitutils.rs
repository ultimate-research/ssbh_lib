use bitvec::prelude::*;
use thiserror::Error;

pub struct BitReader {
    bits: BitVec<u8, Lsb0>,
    index: usize,
}

#[derive(Debug, Error)]
pub enum BitReadError {
    #[error("Failed to read enough bits from reader.")]
    NotEnoughBits,
}

impl BitReader {
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self {
            bits: BitVec::from_slice(bytes),
            index: 0,
        }
    }

    pub fn read_u8(&mut self, bit_count: usize) -> Result<u8, BitReadError> {
        let value: u8 = self
            .bits
            .as_bitslice()
            .get(self.index..self.index + bit_count)
            .ok_or(BitReadError::NotEnoughBits)?
            .load_le();
        self.index += bit_count;

        Ok(value)
    }

    pub fn read_u32(&mut self, bit_count: usize) -> Result<u32, BitReadError> {
        let value: u32 = self
            .bits
            .as_bitslice()
            .get(self.index..self.index + bit_count)
            .ok_or(BitReadError::NotEnoughBits)?
            .load_le();
        self.index += bit_count;

        Ok(value)
    }

    pub fn read_bit(&mut self) -> Result<bool, BitReadError> {
        let value = self
            .bits
            .get(self.index)
            .as_deref()
            .copied()
            .ok_or(BitReadError::NotEnoughBits)?;

        self.index += 1;

        Ok(value)
    }
}

// Assume preallocated sizes for writing bits.
// This requires storing the current index.
// TODO: Find an efficient way to do this with just appending.
pub struct BitWriter {
    bits: BitVec<u8, Lsb0>,
    index: usize,
}

impl BitWriter {
    pub fn new(bits: BitVec<u8, Lsb0>) -> Self {
        Self { bits, index: 0 }
    }

    pub fn write(&mut self, value: u32, bit_count: usize) {
        // TODO: Errors?
        self.bits[self.index..self.index + bit_count].store_le(value);
        self.index += bit_count;
    }

    pub fn write_bit(&mut self, value: bool) {
        // TODO: Errors?
        *self.bits.get_mut(self.index).unwrap() = value;
        self.index += 1;
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bits.into_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bits() {
        let mut reader = BitReader::from_slice(&[0b1011]);
        assert_eq!(true, reader.read_bit().unwrap());
        assert_eq!(true, reader.read_bit().unwrap());
        assert_eq!(false, reader.read_bit().unwrap());
        assert_eq!(true, reader.read_bit().unwrap());
    }

    #[test]
    fn read_u32() {
        let mut reader = BitReader::from_slice(&[3u8, 0u8]);
        assert_eq!(3, reader.read_u32(16).unwrap());
    }

    #[test]
    fn read_bit_past_end() {
        let mut reader = BitReader::from_slice(&[0u8]);
        reader.read_u32(8).unwrap();
        assert!(matches!(
            reader.read_bit(),
            Err(BitReadError::NotEnoughBits)
        ));
    }

    #[test]
    fn read_u32_past_end() {
        let mut reader = BitReader::from_slice(&[0u8]);
        reader.read_bit().unwrap();
        assert!(matches!(
            reader.read_u32(8),
            Err(BitReadError::NotEnoughBits)
        ));
    }

    #[test]
    fn write_bits() {
        let mut bits = BitVec::<u8, Lsb0>::new();
        bits.resize(4, false);
        let mut writer = BitWriter::new(bits);

        writer.write_bit(true);
        writer.write_bit(true);
        writer.write_bit(false);
        writer.write_bit(true);

        assert_eq!(vec![0b1011], writer.into_bytes());
    }

    #[test]
    fn write_u32() {
        let mut bits = BitVec::<u8, Lsb0>::new();
        bits.resize(5, false);
        let mut writer = BitWriter::new(bits);

        writer.write(25, 5);
        assert_eq!(vec![0b11001], writer.into_bytes());
    }

    // TODO: Support writing past end?
}
