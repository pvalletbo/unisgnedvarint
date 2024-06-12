use binrw::{BinRead, BinResult, BinWrite, Endian};
pub struct UnsignedVarint(pub u64);

impl BinRead for UnsignedVarint {
    type Args<'a> = ();

    fn read_options<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        _: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        let mut number = 0;
        // used to track the number of bits that need to be shifted left to place the number from
        // each byte correctly. The last byte we read will be the most significant one becasuse of
        // how the serialization process works. The least significant bytes are encoded first.
        let mut shift_bits = 0;
        // a u64 can be represented with at most 10 bytes. If more than 10 bytes are used it means
        // that there is a problem with the serialized value and an error will be returned
        for _ in 0..10 {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf)?;
            let byte: u64 = buf[0] as u64;
            let is_last_byte = byte & 0b1000_0000 == 0;
            // we set the msb of the new byte to 0, since it is not part of the number to be
            // parsed.
            number |= (byte & 0b0111_1111) << shift_bits;
            shift_bits += 7;

            // If the MSB of the byte is 0 means that there are no more bytes to be read.
            if is_last_byte {
                return Ok(UnsignedVarint(number));
            }
        }
        Err(binrw::Error::Io(std::io::Error::other(
            "u64 can be serialized with at most 10 bytes",
        )))
    }
}

impl BinWrite for UnsignedVarint {
    type Args<'a> = ();

    fn write_options<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _: binrw::Endian,
        _: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<()> {
        let mut number = self.0;

        while number >= 0b1000_0000 {
            // we take only the first 7bits and then we set the 8th bit to 1 to indicate that there
            // are more bytes encoded.
            let part = ((number & 0b0111_1111) as u8) | 0b1000_0000;
            writer.write_all(&[part])?;
            // the 7 first bits are now discarded
            number >>= 7;
        }
        // now the last byte needs to be encoded with the MSB as 0
        writer.write_all(&[(number as u8) & 0b0111_1111])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::{BinReaderExt, BinWriterExt};
    use rstest::rstest;
    use std::io::Cursor;

    #[rstest]
    #[case(&[0xAD, 0x01], 0xAD)]
    #[case(&[128, 0x01], 128)]
    #[case(&[4], 4)]
    #[case(&[128, 8], 1024)]
    #[case(& [255, 0xFF, 255, 255, 15], u32::MAX as u64)]
    #[case(& [0x00], 0)]
    #[case(& [0x01], 1)]
    fn binrw_rw(#[case] bytes: &[u8], #[case] number: u64) {
        // write
        let unsigned_varint = UnsignedVarint(number);
        let mut cursor = Cursor::new(Vec::new());
        cursor.write_be(&unsigned_varint).unwrap();
        assert_eq!(bytes, cursor.into_inner().as_slice());

        // read
        let mut cursor = Cursor::<Vec<u8>>::new(bytes.into());
        let unsigned_varint: UnsignedVarint = cursor.read_be().unwrap();
        assert_eq!(number, unsigned_varint.0);
    }
}
