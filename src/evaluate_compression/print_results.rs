use super::compression_result::{CompressionResult, Finished};

pub fn evaluate_and_printout_results(compression_results: &[CompressionResult<Finished>]) {
    let mut fastest_compression: Option<&CompressionResult<Finished>> = None;
    let mut fastest_decompression: Option<&CompressionResult<Finished>> = None;
    let mut best_ratio: Option<&CompressionResult<Finished>> = None;
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
                br.compression_time.unwrap(),
                br.decompression_time.unwrap(),
                br.compression_ratio.unwrap(),
                br.percentage_of_original.unwrap()
            );
            println!(
                "Best Compression Time:    {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", f_compr.compression_type()),
                f_compr.compression_time.unwrap(),
                f_compr.decompression_time.unwrap(),
                f_compr.compression_ratio.unwrap(),
                f_compr.percentage_of_original.unwrap()
            );
            println!(
                "Best Decompression Time:  {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", f_decompr.compression_type()),
                f_decompr.compression_time.unwrap(),
                f_decompr.decompression_time.unwrap(),
                f_decompr.compression_ratio.unwrap(),
                f_decompr.percentage_of_original.unwrap()
            );
        }
    }
}
