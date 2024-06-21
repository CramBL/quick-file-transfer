use std::ops::RangeInclusive;

use strum_macros::EnumCount;

use super::util::*;

pub const DEFAULT_COMPRESSION_LEVEL: u8 = 6;

#[derive(Debug, Subcommand, Clone, PartialEq, EnumIter, Display, Copy)]
pub enum Compression {
    Bzip2(Bzip2Args),
    Gzip(GzipArgs),
    Lz4,
    Xz(XzArgs),
}

/// This enum exists to be able to specify a variant without specifying arguments, such as with the --omit flag
#[derive(ValueEnum, Debug, Subcommand, Clone, PartialEq, EnumIter, Display, Copy, EnumCount)]
pub enum CompressionVariant {
    Bzip2,
    Gzip,
    Lz4,
    Xz,
}

#[derive(Debug, Args, Clone, PartialEq, Copy)]
#[command(flatten_help = true)]
pub struct GzipArgs {
    /// 0-9: 0=None, 1=Fast, 9=Best
    #[arg(value_parser = clap::value_parser!(u8).range(GzipArgs::range_i64()), default_value_t = DEFAULT_COMPRESSION_LEVEL)]
    pub compression_level: u8,
}

#[derive(Debug, Args, Clone, PartialEq, Copy)]
#[command(flatten_help = true)]
pub struct Bzip2Args {
    /// 0-9: 0=None, 1=Fast, 9=Best
    #[arg(value_parser = clap::value_parser!(u8).range(Bzip2Args::range_i64()), default_value_t = DEFAULT_COMPRESSION_LEVEL)]
    pub compression_level: u8,
}

#[derive(Debug, Args, Clone, PartialEq, Copy)]
#[command(flatten_help = true)]
pub struct XzArgs {
    /// 0-9: 0=None, 1=Fast, 6=default, 9=Best
    #[arg(value_parser = clap::value_parser!(u8).range(XzArgs::range_i64()), default_value_t = DEFAULT_COMPRESSION_LEVEL)]
    pub compression_level: u8,
}

impl Default for Bzip2Args {
    fn default() -> Self {
        Self {
            compression_level: DEFAULT_COMPRESSION_LEVEL,
        }
    }
}

impl Default for GzipArgs {
    fn default() -> Self {
        Self {
            compression_level: DEFAULT_COMPRESSION_LEVEL,
        }
    }
}

impl Default for XzArgs {
    fn default() -> Self {
        Self {
            compression_level: DEFAULT_COMPRESSION_LEVEL,
        }
    }
}

impl From<Compression> for CompressionVariant {
    fn from(value: Compression) -> Self {
        match value {
            Compression::Bzip2(_) => CompressionVariant::Bzip2,
            Compression::Gzip(_) => CompressionVariant::Gzip,
            Compression::Lz4 => CompressionVariant::Lz4,
            Compression::Xz(_) => CompressionVariant::Xz,
        }
    }
}

impl From<&Compression> for &CompressionVariant {
    fn from(value: &Compression) -> Self {
        match value {
            Compression::Bzip2(_) => &CompressionVariant::Bzip2,
            Compression::Gzip(_) => &CompressionVariant::Gzip,
            Compression::Lz4 => &CompressionVariant::Lz4,
            Compression::Xz(_) => &CompressionVariant::Xz,
        }
    }
}

pub trait CompressionRange {
    const DEFAULT_COMPRESSION_LEVEL: u8 = DEFAULT_COMPRESSION_LEVEL;
    const COMPRESSION_LEVEL_RANGE: RangeInclusive<i64> = 1..=9;

    #[must_use]
    fn new(compression_level: u8) -> Self;
    #[must_use]
    fn range_i64() -> RangeInclusive<i64> {
        Self::COMPRESSION_LEVEL_RANGE
    }
    #[must_use]
    fn range_u8() -> RangeInclusive<u8> {
        RangeInclusive::new(
            *Self::range_i64().start() as u8,
            *Self::range_i64().end() as u8,
        )
    }

    /// Omits the provided list of compression levels and returns a list of the rest
    #[must_use]
    fn range_u8_with_omit(omit: &[u8]) -> Vec<u8> {
        Self::range_u8().filter(|l| !omit.contains(l)).collect()
    }
}

impl CompressionRange for XzArgs {
    fn new(compression_level: u8) -> Self {
        Self { compression_level }
    }
}

impl CompressionRange for Bzip2Args {
    fn new(compression_level: u8) -> Self {
        Self { compression_level }
    }
}

impl CompressionRange for GzipArgs {
    fn new(compression_level: u8) -> Self {
        Self { compression_level }
    }
}
