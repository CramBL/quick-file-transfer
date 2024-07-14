use std::{
    io::Read,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
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
use console::Emoji;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
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

    let res = multi_progress_bar(compression_awaiting, &test_contents, threads)?;

    print_results::evaluate_and_printout_results(&res);

    Ok(())
}

fn multi_progress_bar(
    compression_awaiting: Vec<CompressionResult<Awaiting>>,
    test_contents: &Vec<u8>,
    thread_count: usize,
) -> anyhow::Result<Vec<CompressionResult<Finished>>> {
    let multi_bar: MultiProgress = MultiProgress::new();

    let waiting_style =
        ProgressStyle::default_bar().template("{prefix:.bold.dim} {msg:>30.dim}")?;

    let progress_counter = Arc::new(AtomicUsize::new(0));
    let total = compression_awaiting.len();

    // Create and manage progress bars for each task
    let p_bars: Vec<_> = (0..=thread_count)
        .map(|i| {
            if i == thread_count {
                let pb = multi_bar.insert_from_back(thread_count, ProgressBar::new(total as u64));
                pb.set_style(
                    style_global_tracker().expect("Failed to set global progress bar style"),
                );
                pb.set_prefix(prefix_global_tracker(thread_count));
                pb.enable_steady_tick(Duration::from_millis(200));
                (pb, AtomicBool::new(true))
            } else {
                let pb = multi_bar.add(ProgressBar::new(total as u64));
                (pb, AtomicBool::new(false))
            }
        })
        .collect();

    let res = std::thread::scope(|s| {
        let res = s
            .spawn(|| {
                let compression_results: Vec<CompressionResult<Finished>> = compression_awaiting
                    .into_par_iter()
                    .flat_map(|cr_await| {
                        let (pb, is_active) = p_bars
                            .iter()
                            .find(|(_, is_active)| {
                                is_active
                                    .compare_exchange(
                                        false,
                                        true,
                                        Ordering::SeqCst,
                                        Ordering::SeqCst,
                                    )
                                    .is_ok()
                            })
                            .unwrap();
                        pb.set_style(working_style());
                        pb.enable_steady_tick(Duration::from_millis(100));
                        pb.set_message(cr_await.compression.describe_str());

                        let compr_res = cr_await.run(test_contents).ok();
                        if let Some(ref compr_res) = compr_res {
                            {
                                let format = compr_res.compression_format();
                                let mut table: String = compr_res.summarize_as_table();
                                let mut disp_str =
                                    String::with_capacity(format.len() + 1 + table.len());
                                disp_str.push_str(format);
                                disp_str.push('\n');
                                disp_str.extend(table.drain(..));
                                pb.suspend(|| {
                                    log::info!("{disp_str}");
                                })
                            }
                        }
                        let current_progress = progress_counter.fetch_add(1, Ordering::SeqCst);
                        let (global_pb, _) = p_bars.last().unwrap();
                        global_pb.set_position(current_progress as u64 + 1);

                        let items_remaining = total - (current_progress + 1);

                        if items_remaining < thread_count {
                            // Look for progress bars to clean up
                            let mut inactive_count = 0;
                            for (p, is_active) in &p_bars {
                                if !is_active.load(Ordering::SeqCst) {
                                    inactive_count += 1;
                                    if inactive_count > 2 {
                                        is_active.store(true, Ordering::SeqCst);
                                        p.finish_and_clear();
                                    }
                                }
                            }
                            if items_remaining == 0 {
                                pb.finish_and_clear();
                            } else {
                                pb.set_style(waiting_style.clone());
                                pb.disable_steady_tick();
                                pb.set_message(format!("{} waiting...", Emoji("💤 ", "Zzz")));
                                is_active.store(false, Ordering::SeqCst);
                            }
                        } else {
                            pb.reset_elapsed();
                            is_active.store(false, Ordering::SeqCst);
                        }

                        compr_res
                    })
                    .collect();
                compression_results
            })
            .join()
            .unwrap();

        res
    });

    Ok(res)
}

fn style_global_tracker() -> anyhow::Result<ProgressStyle> {
    static CLOCK_12: Emoji<'_, '_> = Emoji("🕛", "⠁");
    static CLOCK_1: Emoji<'_, '_> = Emoji("🕐", "⠂");
    static CLOCK_2: Emoji<'_, '_> = Emoji("🕑", "⠄");
    static CLOCK_3: Emoji<'_, '_> = Emoji("🕒", "⡀");
    static CLOCK_4: Emoji<'_, '_> = Emoji("🕓", "⢀");
    static CLOCK_5: Emoji<'_, '_> = Emoji("🕔", "⠠");
    static CLOCK_6: Emoji<'_, '_> = Emoji("🕕", "⠐");
    static CLOCK_7: Emoji<'_, '_> = Emoji("🕖", "⠈");
    static CLOCK_8: Emoji<'_, '_> = Emoji("🕗", " ");
    static CLOCK_9: Emoji<'_, '_> = Emoji("🕘", "⠁");
    static CLOCK_10: Emoji<'_, '_> = Emoji("🕙", "⠁");
    static CLOCK_11: Emoji<'_, '_> = Emoji("🕚", "⠁");
    let emoji_clock_frames = [
        format!("{}", CLOCK_12),
        format!("{}", CLOCK_1),
        format!("{}", CLOCK_2),
        format!("{}", CLOCK_3),
        format!("{}", CLOCK_4),
        format!("{}", CLOCK_5),
        format!("{}", CLOCK_6),
        format!("{}", CLOCK_7),
        format!("{}", CLOCK_8),
        format!("{}", CLOCK_9),
        format!("{}", CLOCK_10),
        format!("{}", CLOCK_11),
    ];
    let emoji_clock_frames_str: Vec<&str> = emoji_clock_frames.iter().map(|e| e.as_str()).collect();

    Ok(ProgressStyle::default_bar()
        .template(
            "{spinner:.blue} {prefix:.bold.dim} [{elapsed_precise:.bold.dim}]{pos:>3}/{len:3.bold.green}[{wide_bar:.blue}] ({eta})",
        )?
        .progress_chars("##-")
        .tick_strings(&emoji_clock_frames_str))
}

fn prefix_global_tracker(thread_count: usize) -> std::string::String {
    format!(
        "{thread_count} {} (max) {abacus}",
        if thread_count > 1 {
            "workers"
        } else {
            "worker"
        },
        abacus = Emoji("🧮", ""),
    )
}

fn working_style() -> ProgressStyle {
    let moon_frames = ["🌑 ", "🌒 ", "🌓 ", "🌔 ", "🌕 ", "🌖 ", "🌗 ", "🌘 "];
    ProgressStyle::default_bar()
        .template("{spinner:.blue}{elapsed:>4.dim} {msg:>18.bold}")
        .expect("Failed setting progress bar template")
        .tick_strings(&moon_frames)
}
