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

pub fn evaluate_and_printout_results(compression_results: &[CompressionResult]) {
    let mut fastest_compression: Option<&CompressionResult> = None;
    let mut fastest_decompression: Option<&CompressionResult> = None;
    let mut best_ratio: Option<&CompressionResult> = None;
    let results_count = compression_results.len();
    for r in compression_results {
        if fastest_compression.is_none() && results_count > 1 {
            fastest_compression = Some(r);
            fastest_decompression = Some(r);
            best_ratio = Some(r);
        }
        if let Some(f_compr) = fastest_compression {
            if f_compr.compression_time > r.compression_time {
                fastest_compression = Some(r);
            }
        }
        if let Some(f_decompr) = fastest_decompression {
            if f_decompr.decompression_time > r.decompression_time {
                fastest_decompression = Some(r);
            }
        }
        if let Some(br) = best_ratio {
            if br.compressed_size > r.compressed_size {
                best_ratio = Some(r);
            }
        }
        debug_assert!(best_ratio.is_some());
        debug_assert!(fastest_compression.is_some());
        debug_assert!(fastest_decompression.is_some());
    }

    if let (Some(f_compr), Some(f_decompr), Some(br)) =
        (fastest_compression, fastest_decompression, best_ratio)
    {
        println!("===> Summary");
        if f_compr.eq(f_decompr) && f_compr.eq(br) {
            println!("Best in all categories:");
            println!("{}", br.summarize());
        } else {
            println!(
                "Best Compression Ratio:   {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", br.compression_type()),
                br.compression_time,
                br.decompression_time,
                br.compression_ratio,
                br.percentage_of_original
            );
            println!(
                "Best Compression Time:    {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", f_compr.compression_type()),
                f_compr.compression_time,
                f_compr.decompression_time,
                f_compr.compression_ratio,
                f_compr.percentage_of_original
            );
            println!(
                "Best Decompression Time:  {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", f_decompr.compression_type()),
                f_decompr.compression_time,
                f_decompr.decompression_time,
                f_decompr.compression_ratio,
                f_decompr.percentage_of_original
            );
        }
    }
}
