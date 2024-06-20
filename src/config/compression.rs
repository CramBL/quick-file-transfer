use super::util::*;

#[derive(Debug, ValueEnum, Clone, Copy, Display, EnumIter, PartialEq)]
pub enum Compression {
    Gzip,
    Bzip2,
    Xz,
    Lz4,
}
