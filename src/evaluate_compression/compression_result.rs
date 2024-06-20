use std::time::Duration;

use crate::{config::compression::Compression, util::format_data_size};

#[derive(Debug, PartialEq)]
pub struct CompressionResult {
    pub compression: Compression,
    pub compression_time: Duration,
    pub decompression_time: Duration,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub percentage_of_original: f64,
}

impl CompressionResult {
    pub fn new(
        compression: Compression,
        compression_time: Duration,
        decompression_time: Duration,
        compressed_size: usize,
        original_size: usize,
    ) -> Self {
        let compressed_size_f64 = compressed_size as f64;
        let original_size_f64 = original_size as f64;
        let compression_ratio: f64 = original_size_f64 / compressed_size_f64;

        let percentage_of_original = 100. * (compressed_size_f64 / original_size_f64);

        Self {
            compression,
            compression_time,
            decompression_time,
            compressed_size,
            compression_ratio,
            percentage_of_original,
        }
    }

    pub fn compression_type(&self) -> String {
        match self.compression {
            Compression::Bzip2(args) => format!("Bzip2[{}]", args.compression_level),
            Compression::Gzip(args) => format!("Gzip[{}]", args.compression_level),
            Compression::Lz4 => "Lz4".to_string(),
            Compression::Xz(args) => format!("Xz[{}]", args.compression_level),
        }
    }

    pub fn summarize(&self) -> String {
        let mut summary = self.compression_type();
        summary.push('\n');
        summary.push_str(&format!("    Ratio: {:.2}:1\n", self.compression_ratio));
        summary.push_str(&format!(
            "    Compression Time:    {:.2?}\n",
            self.compression_time
        ));
        summary.push_str(&format!(
            "    Decompression Time:  {:.2?}\n",
            self.decompression_time
        ));
        summary.push_str("    Size:  ");
        summary.push_str(&format_data_size(self.compressed_size as u64));
        if self.compressed_size > 1024 {
            summary.push_str(" [");
            summary.push_str(&self.compressed_size.to_string());
            summary.push_str(" B]");
        }
        summary.push_str(&format!(
            " ({:.2}% of original)",
            self.percentage_of_original
        ));
        summary.push('\n');

        summary
    }
}
