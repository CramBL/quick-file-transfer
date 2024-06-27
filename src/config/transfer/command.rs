use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::config::compression::CompressionVariant;

/// Defines commands a QFT client can issue to a QFT server
#[derive(Debug, Serialize, Deserialize, EnumIter)]
#[allow(variant_size_differences)]
pub enum ServerCommand {
    GetFreePort,
    Prealloc(u64),
    ReceiveData(Option<CompressionVariant>),
}

impl ServerCommand {
    /// The length of the command header that describes how long the command is (in bytes).
    ///
    /// # Note
    /// TODO: Revisit this before 1.0
    pub const HEADER_SIZE: usize = 1;

    /// Takes an array of bytes describing the header size and returns how size of the incoming command in bytes
    pub fn size_from_bytes(raw_header: [u8; Self::HEADER_SIZE]) -> usize {
        u8::from_be_bytes(raw_header) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;
    use testresult::TestResult;

    /// Assert that each variant is less than 128 bytes
    #[test]
    fn test_command_serialized_size_constraint() -> TestResult {
        let enum_size = std::mem::size_of::<ServerCommand>();
        eprintln!("Enum size: {enum_size}");
        assert!(enum_size < 255);

        for cmd_variant in ServerCommand::iter() {
            let serialized = bincode::serialize(&cmd_variant)?;
            let serialized_size = serialized.len();
            eprintln!("Serialized {cmd_variant:?} size={serialized_size}");
            assert!(serialized_size < 128);
        }

        Ok(())
    }
}
