use std::{
    hint::black_box,
    io::{self, BufReader, Read},
    time::Instant,
};

use crate::{
    config::{
        compression::{Bzip2Args, Compression, GzipArgs, XzArgs},
        evaluate_compression::EvaluateCompressionArgs,
    },
    send::util::file_with_bufreader,
    util::incremental_rw,
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::{bail, Result};
use strum::IntoEnumIterator;

pub mod compression_result;
use compression_result::CompressionResult;

pub fn evaluate_compression(args: EvaluateCompressionArgs) -> Result<()> {
    let EvaluateCompressionArgs {
        input_file,
        omit,
        mut omit_levels,
    } = args;

    omit_levels.sort_unstable();
    let compression_set: Vec<Compression> = Compression::iter().collect();

    if !omit.is_empty() {
        let mut print_str = String::from("Omitting:  ");
        for compr in &omit {
            print_str.push_str(&format!(" {compr}"));
        }
        eprintln!("{print_str}");
    }

    let evaluate_compressions: Vec<Compression> = compression_set
        .into_iter()
        .filter(|c| !omit.contains(c.into()))
        .collect();

    if !evaluate_compressions.is_empty() {
        let mut print_str = String::from("Evaluating:");
        for compr in &evaluate_compressions {
            print_str.push_str(&format!(" {compr}"));
        }
        eprintln!("{print_str}");
    }

    if !omit_levels.is_empty() {
        let mut print_str = String::from("Omitting compression levels (where applicable):");
        for compr_lvls in &omit_levels {
            print_str.push_str(&format!(" {compr_lvls}"));
        }
        eprintln!("{print_str}");
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

    for compr in &evaluate_compressions {
        test_compress(
            *compr,
            &test_contents,
            test_contents_len,
            &mut compression_results,
            &omit_levels,
        )?;
    }

    compression_result::evaluate_and_printout_results(&compression_results);

    Ok(())
}

fn test_compress(
    compression_ut: Compression,
    test_contents: &Vec<u8>,
    test_contents_len: usize,
    compression_results: &mut Vec<CompressionResult>,
    omit_compression_levels: &[u8],
) -> Result<()> {
    let res_vec: Vec<CompressionResult> = match compression_ut {
        Compression::Bzip2(_) => {
            let mut res_vec = vec![];
            for level in Bzip2Args::range()
                .map(|l| l as u8)
                .filter(|l| !omit_compression_levels.contains(l))
            {
                let mut test_contents_reader = new_bufreader(test_contents);
                let res = black_box(test_compress_bzip2(
                    &mut test_contents_reader,
                    test_contents_len,
                    level,
                ))?;
                res_vec.push(res);
            }
            res_vec
        }
        Compression::Gzip(_) => {
            let mut res_vec = vec![];
            for level in GzipArgs::range()
                .map(|l| l as u8)
                .filter(|l| !omit_compression_levels.contains(l))
            {
                let mut test_contents_reader = new_bufreader(test_contents);
                let res = black_box(test_compress_gzip(
                    &mut test_contents_reader,
                    test_contents_len,
                    level,
                ))?;
                res_vec.push(res);
            }
            res_vec
        }
        Compression::Lz4 => {
            let mut test_contents_reader = new_bufreader(test_contents);
            vec![black_box(test_compress_lz4(
                &mut test_contents_reader,
                test_contents_len,
            ))?]
        }
        Compression::Xz(_) => {
            let mut res_vec = vec![];
            for level in XzArgs::range()
                .map(|l| l as u8)
                .filter(|l| !omit_compression_levels.contains(l))
            {
                let mut test_contents_reader = new_bufreader(test_contents);
                let res = black_box(test_compress_xz(
                    &mut test_contents_reader,
                    test_contents_len,
                    level,
                ))?;
                res_vec.push(res);
            }
            res_vec
        }
    };

    for res in &res_vec {
        eprintln!("{}", res.summarize());
    }
    compression_results.extend(res_vec);

    Ok(())
}

fn test_compress_bzip2(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
    compression_level: u8,
) -> Result<CompressionResult> {
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

    let result = CompressionResult::new(
        Compression::Bzip2(Bzip2Args::new(compression_level)),
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    );

    Ok(result)
}

fn test_compress_gzip(
    test_contents: &mut dyn io::Read,
    test_contents_len: usize,
    compression_level: u8,
) -> Result<CompressionResult> {
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

    let compression_result = CompressionResult::new(
        Compression::Gzip(GzipArgs::new(compression_level)),
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
    compression_level: u8,
) -> Result<CompressionResult> {
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

    let result = CompressionResult::new(
        Compression::Xz(XzArgs::new(compression_level)),
        compress_duration,
        decompress_duration,
        compressed_data.len(),
        test_contents_len,
    );

    Ok(result)
}

fn new_bufreader(test_contents: &Vec<u8>) -> BufReader<&[u8]> {
    BufReader::with_capacity(BUFFERED_RW_BUFSIZE, test_contents.as_slice())
}
