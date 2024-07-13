use std::{hint::black_box, marker::PhantomData, time::Duration};

use super::test_compress::{
    test_compress_bzip2, test_compress_gzip, test_compress_lz4, test_compress_xz,
};
use crate::{config::compression::Compression, util::format_data_size};
use anyhow::Result;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;

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
        match self.compression {
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
        }
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

    pub fn compression_format(&self) -> &str {
        match self.compression {
            Compression::Bzip2(_) => "Bzip2",
            Compression::Gzip(_) => "Gzip",
            Compression::Lz4 => "Lz4",
            Compression::Xz(_) => "Xz",
        }
    }

    pub fn summarize(&self) -> String {
        let mut summary = self.compression_type();
        summary.push_str("\nRatio: ");
        summary.push_str(&self.compression_ratio());
        summary.push_str("\nCompression Time:    ");
        summary.push_str(&self.compression_time());
        summary.push_str("\nDecompression Time:  ");
        summary.push_str(&self.decompression_time());
        summary.push_str("\nSize:  ");
        summary.push_str(&self.compressed_size());
        summary.push_str(" (");
        summary.push_str(&self.percentage_of_original());
        summary.push_str("% of original)");
        summary.push('\n');

        summary
    }

    fn compression_level(&self) -> Option<u8> {
        match self.compression {
            Compression::Bzip2(ref a) => Some(a.compression_level),
            Compression::Gzip(ref a) => Some(a.compression_level),
            Compression::Lz4 => None,
            Compression::Xz(ref a) => Some(a.compression_level),
        }
    }

    fn compression_ratio(&self) -> String {
        format!("{:.2}:1", self.compression_ratio.unwrap())
    }

    fn compression_time(&self) -> String {
        format!("{:.2?}", self.compression_time.unwrap())
    }

    fn decompression_time(&self) -> String {
        format!("{:.2?}", self.decompression_time.unwrap())
    }

    fn compressed_size(&self) -> String {
        let mut compr_size_str = format_data_size(self.compressed_size.unwrap() as u64);
        if self.compressed_size.unwrap() > 1024 {
            compr_size_str.push_str(" [");
            compr_size_str.push_str(&self.compressed_size.unwrap().to_string());
            compr_size_str.push_str(" B]");
        }
        compr_size_str
    }

    fn percentage_of_original(&self) -> String {
        format!("{:.2}%", self.percentage_of_original.unwrap())
    }

    pub fn summarize_as_table(&self) -> String {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);

        if self.compression_level().is_some() {
            table.add_row(vec![
                Self::cell_description_compression_level(),
                self.cell_compression_level().unwrap(),
            ]);
        }
        table.add_row(vec![
            Self::cell_description_compression_ratio(),
            self.cell_compression_ratio(),
        ]);
        table.add_row(vec![
            CompressionResult::<Finished>::cell_description_encode_decode_time(),
            self.cell_encode_decode_time(),
        ]);
        table.add_row(vec![
            CompressionResult::<Finished>::cell_description_compressed_size(),
            self.cell_compressed_size(),
        ]);
        table.add_row(vec![
            CompressionResult::<Finished>::cell_description_percentage_of_original(),
            self.cell_percentage_of_original(),
        ]);

        // Set the default alignment for the third column to right
        let column = table.column_mut(1).expect("Our table has three columns");
        column.set_cell_alignment(CellAlignment::Center);

        table.to_string()
    }

    pub fn cell_description_compressed_size() -> Cell {
        Cell::new("Compressed Size")
    }
    pub fn cell_compressed_size(&self) -> Cell {
        Cell::new(self.compressed_size()).fg(color_grade_0_to_100_green_blue_white_yellow_red(
            self.percentage_of_original.unwrap() as u8,
        ))
    }

    pub fn cell_description_compression_ratio() -> Cell {
        Cell::new("Compression Ratio")
    }
    pub fn cell_compression_ratio(&self) -> Cell {
        Cell::new(self.compression_ratio()).fg(color_grade_0_to_100_green_blue_white_yellow_red(
            self.percentage_of_original.unwrap() as u8,
        ))
    }

    pub fn cell_description_compression_level() -> Cell {
        Cell::new("Compression level")
    }
    pub fn cell_compression_level(&self) -> Option<Cell> {
        self.compression_level().map(|compr_level| {
            Cell::new(compr_level).fg(color_grade_0_to_9_white_to_red(compr_level))
        })
    }
    pub fn cell_description_percentage_of_original() -> Cell {
        Cell::new("% of Original")
    }
    pub fn cell_percentage_of_original(&self) -> Cell {
        Cell::new(self.percentage_of_original()).fg(
            color_grade_0_to_100_green_blue_white_yellow_red(
                self.percentage_of_original.unwrap() as u8
            ),
        )
    }

    pub fn cell_description_encode_decode_time() -> Cell {
        Cell::new("Encode/decode time")
    }
    pub fn cell_encode_decode_time(&self) -> Cell {
        Cell::new(format!(
            "{}/{}",
            &self.compression_time(),
            &self.decompression_time()
        ))
    }
}

/// Color grades from 0-9:
/// * `0`: white
/// * `1-3`: bluer/cyan
/// * `4-8`: cyan -> green -> yellow
/// * `9`: red
pub fn color_grade_0_to_9_white_to_red(val: u8) -> comfy_table::Color {
    debug_assert!(val <= 9);
    match val {
        1..=3 => comfy_table::Color::Rgb {
            r: 255 - 70 * val,
            g: 255,
            b: 255,
        },
        4..=8 => comfy_table::Color::Rgb {
            r: 150 + 20 * (val - 3),
            g: 255,
            b: 255 - 25 * (val - 3),
        },
        9 => comfy_table::Color::Rgb {
            r: 255,
            g: 150,
            b: 150,
        },
        _ => unreachable!(),
    }
}

pub fn color_grade_0_to_100_green_blue_white_yellow_red(val: u8) -> comfy_table::Color {
    debug_assert!(val <= 100);

    if val <= 50 {
        // Transition from green to cyan to blue to white at 50%
        let (r, g, b) = match val {
            0..=34 => {
                // Green to Cyan
                let r = 0;
                let g = 255;
                let b = (val as f32 * 6.2) as u8; // 0 to 255
                (r, g, b)
            }
            35..=50 => {
                // Cyan to Blue to White
                let r = ((val - 25) as f32 * 10.2) as u8; // 0 to 255
                let g = 255;
                let b = 255;
                (r, g, b)
            }
            _ => unreachable!(),
        };
        comfy_table::Color::Rgb { r, g, b }
    } else if val <= 80 {
        // Transition from white at 50% to yellow at 80%
        let r = 255;
        let g = 255;
        let b = 255 - ((val - 50) as f32 * 8.5) as u8; // 255 to 0
        comfy_table::Color::Rgb { r, g, b }
    } else {
        // Transition from yellow at 80% to red at 100%
        let r = 255;
        let g = 255 - ((val - 80) as f32 * 12.75) as u8; // 255 to 0
        let b = 0;
        comfy_table::Color::Rgb { r, g, b }
    }
}

pub fn print_results_as_table(
    fastest_compr: &CompressionResult<Finished>,
    fastest_decompr: &CompressionResult<Finished>,
    best_ratio: &CompressionResult<Finished>,
) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "",
        "Best Ratio",
        "Best Compression Time",
        "Best Decompression Time",
    ]);
    table.add_row(vec![
        "Format",
        best_ratio.compression_format(),
        fastest_compr.compression_format(),
        fastest_decompr.compression_format(),
    ]);
    table.add_row(vec![
        CompressionResult::<Finished>::cell_description_compression_level(),
        best_ratio
            .cell_compression_level()
            .unwrap_or(Cell::new("-")),
        fastest_compr
            .cell_compression_level()
            .unwrap_or(Cell::new("-")),
        fastest_decompr
            .cell_compression_level()
            .unwrap_or(Cell::new("-")),
    ]);
    table.add_row(vec![
        CompressionResult::<Finished>::cell_description_compression_ratio(),
        best_ratio.cell_compression_ratio(),
        fastest_compr.cell_compression_ratio(),
        fastest_decompr.cell_compression_ratio(),
    ]);
    table.add_row(vec![
        CompressionResult::<Finished>::cell_description_encode_decode_time(),
        best_ratio.cell_encode_decode_time(),
        fastest_compr.cell_encode_decode_time(),
        fastest_decompr.cell_encode_decode_time(),
    ]);
    table.add_row(vec![
        CompressionResult::<Finished>::cell_description_compressed_size(),
        best_ratio.cell_compressed_size(),
        fastest_compr.cell_compressed_size(),
        fastest_decompr.cell_compressed_size(),
    ]);
    table.add_row(vec![
        CompressionResult::<Finished>::cell_description_percentage_of_original(),
        best_ratio.cell_percentage_of_original(),
        fastest_compr.cell_percentage_of_original(),
        fastest_decompr.cell_percentage_of_original(),
    ]);

    println!("{table}");
    println!("\n==> Short summary");
    println!(
                "Best Compression Ratio:   {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", best_ratio.compression_type()),
                best_ratio.compression_time.unwrap(),
                best_ratio.decompression_time.unwrap(),
                best_ratio.compression_ratio.unwrap(),
                best_ratio.percentage_of_original.unwrap()
            );
    println!(
                "Best Compression Time:    {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", fastest_compr.compression_type()),
                fastest_compr.compression_time.unwrap(),
                fastest_compr.decompression_time.unwrap(),
                fastest_compr.compression_ratio.unwrap(),
                fastest_compr.percentage_of_original.unwrap()
            );
    println!(
                "Best Decompression Time:  {:<8} Compression/Decompression: {:>10.2?}/{:>10.2?} {:>6.2}:1 ({:>4.2}% of original)",
                format!("{}", fastest_decompr.compression_type()),
                fastest_decompr.compression_time.unwrap(),
                fastest_decompr.decompression_time.unwrap(),
                fastest_decompr.compression_ratio.unwrap(),
                fastest_decompr.percentage_of_original.unwrap()
            );
}

#[cfg(test)]
mod tests {
    use crate::evaluate_compression::compression_result::color_grade_0_to_100_green_blue_white_yellow_red;
    use comfy_table::*;

    #[test]
    fn tune_color_grade_0_to_100_green_blue_white_yellow_red() {
        let mut table = Table::new();
        let mut cells = vec![];
        for x in 0..=100 {
            let c =
                comfy_table::Cell::new(x).fg(color_grade_0_to_100_green_blue_white_yellow_red(x));
            cells.push(c);

            if cells.len() == 26 {
                table.add_row(cells);
                cells = vec![];
            }
        }
        table.add_row(cells);
        println!("{table}");
    }
}
