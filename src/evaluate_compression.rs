use std::{
    hint::black_box,
    io::{self, BufReader, Read},
    time::{Duration, Instant},
};

use crate::{
    config::{evaluate_compression::EvaluateCompressionArgs, Compression},
    send::util::file_with_bufreader,
    util::{format_data_size, incremental_rw},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::{bail, Result};
use strum::IntoEnumIterator;

pub fn evaluate_compression(args: EvaluateCompressionArgs) -> Result<()> {
    let EvaluateCompressionArgs {
        input_file,
        omit,
        test_mmap,
    } = args;

    let compression_set: Vec<Compression> = Compression::iter().collect();

    for compr in omit.iter() {
        eprintln!("Omitting: {compr}");
    }

    let evaluate_compressions: Vec<Compression> = compression_set
        .into_iter()
        .filter(|c| !omit.contains(c))
        .collect();

    for compr in evaluate_compressions.iter() {
        eprintln!("evaluating: {compr}");
    }

    let mut bufreader = file_with_bufreader(&input_file)?;

    let start = Instant::now();
    let mut test_contents = Vec::new();
    bufreader.read_to_end(&mut test_contents)?;
    let elapsed = start.elapsed();
    let test_contents_len = test_contents.len();
    if test_contents_len == 0 {
        bail!("Invalid content size of 0, please provide a non-empty file")
    }
    eprintln!("Buffered reading {test_contents_len} B contents in {elapsed:?}",);
    let mut compression_results: Vec<CompressionResult> = Vec::new();

    for compr in evaluate_compressions.iter() {
        let mut test_contents_reader =
            BufReader::with_capacity(BUFFERED_RW_BUFSIZE, test_contents.as_slice());
        test_compress(
            *compr,
            &mut test_contents_reader,
            test_contents_len,
            &mut compression_results,
        )?;
    }

    if test_mmap {
        todo!("Implement evaluation for mmapping");
        //let mmap_read = MemoryMappedReader::new(iinput_file)?;
    }

    let mut fastest_compression: Option<&CompressionResult> = None;
    let mut fastest_decompression: Option<&CompressionResult> = None;
    let mut best_ratio: Option<&CompressionResult> = None;
    let results_count = compression_results.len();
    for r in compression_results.iter() {
        if fastest_compression.is_none() && results_count > 1 {
            fastest_compression = Some(r);
            fastest_decompression = Some(r);
            best_ratio = Some(r);
        }
        if let Some(f_compr) = fastest_compression {
            if f_compr.compression_time > r.compression_time {
                fastest_compression = Some(r);
            } else {
                fastest_compression = Some(f_compr);
            }
        }
        if let Some(f_decompr) = fastest_decompression {
            if f_decompr.decompression_time > r.decompression_time {
                fastest_decompression = Some(r);
            } else {
                fastest_decompression = Some(f_decompr);
            }
        }
        if let Some(br) = best_ratio {
            if br.compressed_size > r.compressed_size {
                best_ratio = Some(r);
            } else {
                best_ratio = Some(br);
            }
        }
    }

    if let (Some(f_compr), Some(f_decompr), Some(br)) =
        (fastest_compression, fastest_decompression, best_ratio)
    {
        eprintln!("===> Summary");
        if f_compr.eq(f_decompr) && f_compr.eq(br) {
            eprintln!("Best in all categories:");
            eprintln!("{}", br.summarize());
        } else {
            eprintln!(
                "Best Compression Ratio:   {:<4} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{:?}", br.compression),
                br.compression_time,
                br.decompression_time,
                br.compression_ratio,
                br.percentage_of_original
            );
            eprintln!(
                "Best Compression Time:    {:<4} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{:?}", f_compr.compression),
                f_compr.compression_time,
                f_compr.decompression_time,
                f_compr.compression_ratio,
                f_compr.percentage_of_original
            );
            eprintln!(
                "Best Decompression Time:  {:<4} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{:?}", f_decompr.compression),
                f_decompr.compression_time,
                f_decompr.decompression_time,
                f_decompr.compression_ratio,
                f_decompr.percentage_of_original
            );
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
struct CompressionResult {
    pub compression: Compression,
    pub compression_time: Duration,
    pub decompression_time: Duration,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub percentage_of_original: f64,
}

impl CompressionResult {
    pub fn new(
        ty: Compression,
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
            compression: ty,
            compression_time,
            decompression_time,
            compressed_size,
            compression_ratio,
            percentage_of_original,
        }
    }

    pub fn summarize(&self) -> String {
        let mut summary = self.compression.to_string();
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

fn test_compress(
    compression_ut: Compression,
    test_contents_reader: &mut dyn io::Read,
    test_contents_len: usize,
    compression_results: &mut Vec<CompressionResult>,
) -> Result<()> {
    let res = match compression_ut {
        Compression::Gzip => black_box(test_compress_gzip(test_contents_reader, test_contents_len)),
        Compression::Bzip2 => todo!(),
        Compression::Xz => black_box(test_compress_xz(test_contents_reader, test_contents_len)),
        Compression::Lz4 => black_box(test_compress_lz4(test_contents_reader, test_contents_len)),
        Compression::None => unreachable!("nonsense"),
    }?;

    eprintln!("{}", res.summarize());
    compression_results.push(res);

    Ok(())
}

fn test_compress_gzip(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
) -> Result<CompressionResult> {
    use flate2::read::{GzDecoder, GzEncoder};
    let mut compressed_data: Vec<u8> = Vec::new();

    // Compress
    let start = Instant::now();
    let mut gz_encoder = GzEncoder::new(test_contents, flate2::Compression::default());
    let _total_read = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut compressed_data, &mut gz_encoder)?;
    let compress_duration = start.elapsed();

    // Decompress
    let mut decompressed_data = Vec::new();
    let start = Instant::now();
    let mut gz_decoder = GzDecoder::new(compressed_data.as_slice());
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut decompressed_data, &mut gz_decoder)?;
    let decompress_duration = start.elapsed();

    let compression_result = CompressionResult::new(
        Compression::Gzip,
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    );

    Ok(compression_result)
}

fn test_compress_lz4(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
) -> Result<CompressionResult> {
    let mut compressed_data: Vec<u8> = Vec::new();

    // Compress
    let start = Instant::now();
    let mut lz4_encoder = lz4_flex::frame::FrameEncoder::new(&mut compressed_data);
    let _total_read = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut lz4_encoder, test_contents)?;
    lz4_encoder.finish()?;
    let compress_duration = start.elapsed();
    let compressed_size = compressed_data.len();

    // Decompress
    let mut decompressed_data = Vec::new();
    let start = Instant::now();
    let mut lz4_decoder = lz4_flex::frame::FrameDecoder::new(compressed_data.as_slice());
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut decompressed_data, &mut lz4_decoder)?;
    let decompress_duration = start.elapsed();

    let result = CompressionResult::new(
        Compression::Lz4,
        compress_duration,
        decompress_duration,
        compressed_size,
        test_contents_len,
    );

    Ok(result)
}

fn test_compress_xz(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
) -> Result<CompressionResult> {
    let mut compressed_data: Vec<u8> = Vec::new();

    // Compress
    let start = Instant::now();
    let mut xz_encoder = xz2::read::XzEncoder::new(test_contents, 9);
    let _total_read = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut compressed_data, &mut xz_encoder)?;
    let compress_duration = start.elapsed();

    // Decompress
    let mut decompressed_data = Vec::new();
    let start = Instant::now();
    let mut xz_decoder = xz2::read::XzDecoder::new(compressed_data.as_slice());
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut decompressed_data, &mut xz_decoder)?;
    let decompress_duration = start.elapsed();

    let result = CompressionResult::new(
        Compression::Xz,
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    );

    Ok(result)
}
