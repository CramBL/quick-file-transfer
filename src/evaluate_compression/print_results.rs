use super::compression_result::{print_results_as_table, CompressionResult, Finished};

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
        print_results_as_table(f_compr, f_decompr, br);
    }
}
