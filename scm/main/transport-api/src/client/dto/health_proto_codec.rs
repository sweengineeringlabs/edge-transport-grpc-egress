//! Minimal protobuf wire helpers for the standard gRPC health messages.

use proto_pattern::DecodeError;

pub(super) struct HealthProtoCodec;

impl HealthProtoCodec {
    /// Creates a reader for one named health payload.
    pub(super) fn reader<'a>(bytes: &'a [u8], message: &'static str) -> HealthProtoReader<'a> {
        HealthProtoReader {
            bytes,
            offset: 0,
            message,
        }
    }

    /// Appends one protobuf varint to `out`.
    pub(super) fn write_varint(out: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            out.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    /// Returns the number of bytes in the protobuf varint encoding of `value`.
    pub(super) fn varint_len(mut value: u64) -> usize {
        let mut length = 1;
        while value >= 0x80 {
            value >>= 7;
            length += 1;
        }
        length
    }
}

/// Reads protobuf fields from a health message.
pub(super) struct HealthProtoReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    message: &'static str,
}

impl HealthProtoReader<'_> {
    /// Reads the next field number and wire type, or returns `None` at the end.
    pub(super) fn next_field(&mut self) -> Result<Option<(u32, u8)>, DecodeError> {
        if self.offset == self.bytes.len() {
            return Ok(None);
        }
        let key = self.read_varint()?;
        let field = key >> 3;
        let wire_type = (key & 0x07) as u8;
        if field == 0 || field > 0x1fff_ffff || matches!(wire_type, 3 | 4 | 6 | 7) {
            return Err(self.error("invalid protobuf field key"));
        }
        Ok(Some((field as u32, wire_type)))
    }

    /// Reads one protobuf varint.
    pub(super) fn read_varint(&mut self) -> Result<u64, DecodeError> {
        let mut value = 0u64;
        for index in 0..10 {
            let Some(byte) = self.bytes.get(self.offset).copied() else {
                return Err(self.error("truncated protobuf varint"));
            };
            self.offset += 1;
            if index == 9 && byte > 1 {
                return Err(self.error("protobuf varint overflows 64 bits"));
            }
            value |= u64::from(byte & 0x7f) << (index * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(self.error("protobuf varint exceeds 10 bytes"))
    }

    /// Reads one length-delimited field.
    pub(super) fn read_length_delimited(&mut self) -> Result<&[u8], DecodeError> {
        let length = usize::try_from(self.read_varint()?)
            .map_err(|_| self.error("protobuf length does not fit in memory"))?;
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| self.error("protobuf length overflows"))?;
        let Some(value) = self.bytes.get(self.offset..end) else {
            return Err(self.error("truncated length-delimited protobuf field"));
        };
        self.offset = end;
        Ok(value)
    }

    /// Skips one unknown field of `wire_type`.
    pub(super) fn skip(&mut self, wire_type: u8) -> Result<(), DecodeError> {
        match wire_type {
            0 => self.read_varint().map(|_| ()),
            1 => self.skip_bytes(8),
            2 => self.read_length_delimited().map(|_| ()),
            5 => self.skip_bytes(4),
            _ => Err(self.error("unsupported protobuf wire type")),
        }
    }

    fn skip_bytes(&mut self, length: usize) -> Result<(), DecodeError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| self.error("protobuf field length overflows"))?;
        if end > self.bytes.len() {
            return Err(self.error("truncated fixed-width protobuf field"));
        }
        self.offset = end;
        Ok(())
    }

    /// Creates a decode error scoped to this message.
    pub(super) fn error(&self, reason: &'static str) -> DecodeError {
        DecodeError::create(self.message, reason)
    }
}
