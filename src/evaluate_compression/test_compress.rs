use crate::{
    config::compression::{Bzip2Args, Compression, CompressionRange, GzipArgs, XzArgs},
    util::incremental_rw,
    TCP_STREAM_BUFSIZE,
};
use anyhow::Result;
use std::{io, time::Instant};

use super::compression_result::{CompressionResult, Finished};

pub fn test_compress_bzip2(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
    compression_level: u8,
) -> Result<CompressionResult<Finished>> {
    use bzip2::read::{BzDecoder, BzEncoder};

    let mut compressed_data: Vec<u8> = Vec::new();
    let level = bzip2::Compression::new(compression_level.into());

    // Compress
    let start = Instant::now();
    let mut bzip2_encoder = BzEncoder::new(test_contents, level);
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut compressed_data, &mut bzip2_encoder)?;
    let compress_duration = start.elapsed();

    // Decompress
    let mut decompressed_data = Vec::new();
    let start = Instant::now();
    let mut bzip2_decoder = BzDecoder::new(compressed_data.as_slice());
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut decompressed_data, &mut bzip2_decoder)?;
    let decompress_duration = start.elapsed();

    Ok(CompressionResult::conclude(
        Compression::Bzip2(Bzip2Args::new(compression_level)),
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    ))
}

pub fn test_compress_gzip(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
    compression_level: u8,
) -> Result<CompressionResult<Finished>> {
    use flate2::read::{GzDecoder, GzEncoder};
    let mut compressed_data: Vec<u8> = Vec::new();

    // Compress
    let start = Instant::now();
    let mut gz_encoder = GzEncoder::new(
        test_contents,
        flate2::Compression::new(compression_level.into()),
    );
    let _total_read = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut compressed_data, &mut gz_encoder)?;
    let compress_duration = start.elapsed();

    // Decompress
    let mut decompressed_data = Vec::new();
    let start = Instant::now();
    let mut gz_decoder = GzDecoder::new(compressed_data.as_slice());
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut decompressed_data, &mut gz_decoder)?;
    let decompress_duration = start.elapsed();

    Ok(CompressionResult::conclude(
        Compression::Gzip(GzipArgs::new(compression_level)),
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    ))
}

pub fn test_compress_lz4(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
) -> Result<CompressionResult<Finished>> {
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

    Ok(CompressionResult::conclude(
        Compression::Lz4,
        compress_duration,
        decompress_duration,
        compressed_size,
        test_contents_len,
    ))
}

pub fn test_compress_xz(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
    compression_level: u8,
) -> Result<CompressionResult<Finished>> {
    let mut compressed_data: Vec<u8> = Vec::new();

    // Compress
    let start = Instant::now();
    let mut xz_encoder = xz2::read::XzEncoder::new(test_contents, compression_level.into());
    let _total_read = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut compressed_data, &mut xz_encoder)?;
    let compress_duration = start.elapsed();

    // Decompress
    let mut decompressed_data = Vec::new();
    let start = Instant::now();
    let mut xz_decoder = xz2::read::XzDecoder::new(compressed_data.as_slice());
    let _total_read =
        incremental_rw::<TCP_STREAM_BUFSIZE>(&mut decompressed_data, &mut xz_decoder)?;
    let decompress_duration = start.elapsed();

    Ok(CompressionResult::conclude(
        Compression::Xz(XzArgs::new(compression_level)),
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    ))
}
