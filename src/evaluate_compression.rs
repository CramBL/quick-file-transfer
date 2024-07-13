use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use crate::{
    config::{
        compression::{
            Bzip2Args, Compression, CompressionRange, CompressionVariant, GzipArgs, XzArgs,
        },
        evaluate_compression::EvaluateCompressionArgs,
    },
    send::util::file_with_bufreader,
};
use anyhow::{bail, Result};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use strum::IntoEnumIterator;

pub mod compression_result;
use compression_result::{Awaiting, CompressionResult, Finished};

mod print_results;
mod test_compress;

pub fn evaluate_compression(args: EvaluateCompressionArgs) -> Result<()> {
    let EvaluateCompressionArgs {
        input_file,
        omit,
        mut omit_levels,
        threads,
    } = args;

    omit_levels.sort_unstable();
    let compression_set: Vec<Compression> = Compression::iter().collect();

    if !omit.is_empty() {
        let mut print_str = String::from("Omitting:  ");
        for compr in &omit {
            print_str.push_str(&format!(" {compr}"));
        }
        log::info!("{print_str}");
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
        log::info!("{print_str}");
    }

    if !omit_levels.is_empty() {
        let mut print_str = String::from("Omitting compression levels (where applicable):");
        for compr_lvls in &omit_levels {
            print_str.push_str(&format!(" {compr_lvls}"));
        }
        log::info!("{print_str}");
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
    log::info!("Buffered reading {test_contents_len} B contents in {elapsed:?}");

    let mut compression_awaiting: Vec<CompressionResult<Awaiting>> = Vec::new();

    if evaluate_compressions.contains(&Compression::Lz4) {
        compression_awaiting.push(CompressionResult::new(Compression::Lz4));
    }

    if !omit.contains(&CompressionVariant::Bzip2) {
        for compression_level in <Bzip2Args>::range_u8_with_omit(&omit_levels) {
            compression_awaiting.push(CompressionResult::new(Compression::Bzip2(Bzip2Args {
                compression_level,
            })));
        }
    }
    if !omit.contains(&CompressionVariant::Gzip) {
        for compression_level in <GzipArgs>::range_u8_with_omit(&omit_levels) {
            compression_awaiting.push(CompressionResult::new(Compression::Gzip(GzipArgs {
                compression_level,
            })));
        }
    }
    if !omit.contains(&CompressionVariant::Xz) {
        for compression_level in <XzArgs>::range_u8_with_omit(&omit_levels) {
            compression_awaiting.push(CompressionResult::new(Compression::Xz(XzArgs {
                compression_level,
            })));
        }
    }

    log::info!(
        "Evaluating {} compression combinations",
        compression_awaiting.len()
    );
    if threads == 1 {
        log::info!("Running sequentially on the main thread");
    } else {
        log::info!("Running sequentially with up to {threads} threads");
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;

    // Setup the progress bar
    let res = single_progress_bar(compression_awaiting, &test_contents)?;

    print_results::evaluate_and_printout_results(&res);

    Ok(())
}

fn single_progress_bar(
    compression_awaiting: Vec<CompressionResult<Awaiting>>,
    test_contents: &Vec<u8>,
) -> anyhow::Result<Vec<CompressionResult<Finished>>> {
    use indicatif::style::*;
    use indicatif::*;
    let total = compression_awaiting.len();
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({eta})",
        )?
        .progress_chars("##-"),
    );

    // Atomic counter for tracking progress in the parallel iterator
    let progress_counter = Arc::new(AtomicUsize::new(0));

    let res = std::thread::scope(|s| {
        let res = s.spawn(|| {
            let compression_results: Vec<CompressionResult<Finished>> = compression_awaiting
                .into_par_iter()
                .flat_map(|cr_await| {
                    let compr_res = cr_await.run(test_contents).ok();
                    if let Some(ref compr_res) = compr_res {
                        pb.suspend(|| {
                            log::info!(
                                "{}\n{}",
                                compr_res.compression_format(),
                                compr_res.summarize_as_table()
                            );
                        })
                    }
                    let current_progress = progress_counter.fetch_add(1, Ordering::SeqCst);
                    pb.set_position(current_progress as u64 + 1);
                    compr_res
                })
                .collect();
            compression_results
        });

        let res = res.join().unwrap();
        pb.finish_with_message("Processing complete");
        res
    });

    Ok(res)
}
