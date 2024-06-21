use std::{hint::black_box, marker::PhantomData, time::Duration};

use super::test_compress::{
    test_compress_bzip2, test_compress_gzip, test_compress_lz4, test_compress_xz,
};
use crate::{config::compression::Compression, util::format_data_size};
use anyhow::Result;

#[derive(Debug, PartialEq)]
pub struct Awaiting;
#[derive(Debug, PartialEq)]
pub struct Finished;
pub trait CompressionResultState {}

impl CompressionResultState for Awaiting {}
impl CompressionResultState for Finished {}

#[derive(Debug, PartialEq)]
pub struct CompressionResult<S: CompressionResultState> {
    pub compression: Compression,
    pub compression_time: Option<Duration>,
    pub decompression_time: Option<Duration>,
    pub compressed_size: Option<usize>,
    pub compression_ratio: Option<f64>,
    pub percentage_of_original: Option<f64>,
    state: PhantomData<S>,
}

impl CompressionResult<Awaiting> {
    pub fn new(compression: Compression) -> Self {
        Self {
            compression,
            state: PhantomData,
            compression_time: None,
            decompression_time: None,
            compressed_size: None,
            compression_ratio: None,
            percentage_of_original: None,
        }
    }

    pub fn run(self, test_contents: &Vec<u8>) -> Result<CompressionResult<Finished>> {
        let mut bufread = new_bufreader(test_contents);
        let res = match self.compression {
            Compression::Bzip2(a) => black_box(test_compress_bzip2(
                &mut bufread,
                test_contents.len(),
                a.compression_level,
            )),
            Compression::Gzip(a) => black_box(test_compress_gzip(
                &mut bufread,
                test_contents.len(),
                a.compression_level,
            )),
            Compression::Lz4 => black_box(test_compress_lz4(&mut bufread, test_contents.len())),
            Compression::Xz(a) => black_box(test_compress_xz(
                &mut bufread,
                test_contents.len(),
                a.compression_level,
            )),
        };
        if let Ok(ref res) = res {
            log::info!("{}", res.summarize());
        }
        res
    }
}

fn new_bufreader(test_contents: &Vec<u8>) -> std::io::BufReader<&[u8]> {
    std::io::BufReader::with_capacity(crate::BUFFERED_RW_BUFSIZE, test_contents.as_slice())
}

impl CompressionResult<Finished> {
    pub fn conclude(
        compression: Compression,
        compression_time: Duration,
        decompression_time: Duration,
        compressed_size: usize,
        original_size: usize,
    ) -> Self {
        let compressed_size_f64 = compressed_size as f64;
        let original_size_f64 = original_size as f64;
        let compression_ratio: f64 = original_size_f64 / compressed_size_f64;
        let percentage_of_original: f64 = 100. * (compressed_size_f64 / original_size_f64);

        Self {
            compression,
            compression_time: Some(compression_time),
            decompression_time: Some(decompression_time),
            compressed_size: Some(compressed_size),
            compression_ratio: Some(compression_ratio),
            percentage_of_original: Some(percentage_of_original),
            state: PhantomData,
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
        summary.push_str(&format!(
            "Ratio: {:.2}:1\n",
            self.compression_ratio.unwrap()
        ));
        summary.push_str(&format!(
            "Compression Time:    {:.2?}\n",
            self.compression_time.unwrap()
        ));
        summary.push_str(&format!(
            "Decompression Time:  {:.2?}\n",
            self.decompression_time.unwrap()
        ));
        summary.push_str("Size:  ");
        summary.push_str(&format_data_size(self.compressed_size.unwrap() as u64));
        if self.compressed_size.unwrap() > 1024 {
            summary.push_str(" [");
            summary.push_str(&self.compressed_size.unwrap().to_string());
            summary.push_str(" B]");
        }
        summary.push_str(&format!(
            " ({:.2}% of original)",
            self.percentage_of_original.unwrap()
        ));
        summary.push('\n');

        summary
    }
}
