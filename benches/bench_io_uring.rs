use criterion::{criterion_group, criterion_main, Criterion};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fs::File, io::Write};
use std::{io, thread};

// Assuming these functions are in your crate
use quick_file_transfer::io_uring::{
    incremental_rw_io_uring_batch, incremental_rw_io_uring_batch_alt,
    incremental_rw_io_uring_batch_alt2, incremental_rw_io_uring_simple,
};

// Helper function to start a background thread that reads and discards data from the TCP socket
fn start_tcp_sinkhole(listener: Arc<Mutex<TcpListener>>) {
    thread::spawn(move || {
        let listener = listener.lock().unwrap();
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = std::io::copy(&mut stream, &mut io::sink()).unwrap(); // Read and discard
        }
    });
}

// Helper function to create a TcpStream and a file path for testing
fn setup() -> (TcpStream, PathBuf) {
    // Setup TCP listener and connect to it
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind listener");
    let listener_addr = listener.local_addr().unwrap();
    let listener = Arc::new(Mutex::new(listener));
    start_tcp_sinkhole(listener.clone());

    let socket = TcpStream::connect(listener_addr).expect("Failed to connect to listener");
    let file_path = std::env::current_dir().unwrap().join("large_file.bin");

    create_large_file(&file_path, 100).expect("Failed to create large file"); // 100 MB file
    (socket, file_path)
}

fn create_large_file(file_path: &Path, size_mb: usize) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    let data = (0..123456).map(|i| (i % 256) as u8).collect::<Vec<u8>>();

    let mut len = data.len();
    while len < 1024 * 1024 * size_mb {
        file.write_all(&data)?;
        len += data.len();
    }

    Ok(())
}

// Benchmark for `incremental_rw_io_uring_simple`
fn benchmark_incremental_rw_io_uring_simple(c: &mut Criterion) {
    let (socket, file_path) = setup();
    c.bench_function("incremental_rw_io_uring_simple", |b| {
        b.iter(|| incremental_rw_io_uring_simple::<4096>(&socket, &file_path).unwrap())
    });
}

// Benchmark for `incremental_rw_io_uring_batch`
fn benchmark_incremental_rw_io_uring_batch(c: &mut Criterion) {
    let (socket, file_path) = setup();
    c.bench_function("incremental_rw_io_uring_batch", |b| {
        b.iter(|| incremental_rw_io_uring_batch::<4096, 8>(&socket, &file_path).unwrap())
    });
}

// Benchmark for `incremental_rw_io_uring_batch_alt`
fn benchmark_incremental_rw_io_uring_batch_alt<const BUFSIZE: usize, const BATCH_COUNT: usize>(
    c: &mut Criterion,
) {
    let (socket, file_path) = setup();
    c.bench_function("incremental_rw_io_uring_batch_alt", |b| {
        b.iter(|| {
            incremental_rw_io_uring_batch_alt::<BUFSIZE, BATCH_COUNT>(&socket, &file_path).unwrap()
        })
    });
}

// Benchmark for `incremental_rw_io_uring_batch_alt2`
fn benchmark_incremental_rw_io_uring_batch_alt2<const BUFSIZE: usize, const BATCH_COUNT: usize>(
    c: &mut Criterion,
) {
    let (socket, file_path) = setup();
    c.bench_function("incremental_rw_io_uring_batch_alt2", |b| {
        b.iter(|| incremental_rw_io_uring_batch_alt2::<4096, 8>(&socket, &file_path).unwrap())
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = benchmark_incremental_rw_io_uring_simple,
              benchmark_incremental_rw_io_uring_batch,
              benchmark_incremental_rw_io_uring_batch_alt::<2048, 16>,
              benchmark_incremental_rw_io_uring_batch_alt::<4096, 16>,
              benchmark_incremental_rw_io_uring_batch_alt::<4096, 32>,
              benchmark_incremental_rw_io_uring_batch_alt::<4096, 64>,
              benchmark_incremental_rw_io_uring_batch_alt::<4096, 128>,
              benchmark_incremental_rw_io_uring_batch_alt::<4096, 256>,
              benchmark_incremental_rw_io_uring_batch_alt::<8192, 16>,
              benchmark_incremental_rw_io_uring_batch_alt::<8192, 32>,
              benchmark_incremental_rw_io_uring_batch_alt::<8192, 64>,
              benchmark_incremental_rw_io_uring_batch_alt::<8192, 128>,
              benchmark_incremental_rw_io_uring_batch_alt::<8192, 256>,
              benchmark_incremental_rw_io_uring_batch_alt::<16384, 16>,
              benchmark_incremental_rw_io_uring_batch_alt::<16384, 32>,
              benchmark_incremental_rw_io_uring_batch_alt::<16384, 64>,
              benchmark_incremental_rw_io_uring_batch_alt::<16384, 128>,
              benchmark_incremental_rw_io_uring_batch_alt::<16384, 256>,
              benchmark_incremental_rw_io_uring_batch_alt2::<2048, 16>,
              benchmark_incremental_rw_io_uring_batch_alt2::<4096, 16>,
              benchmark_incremental_rw_io_uring_batch_alt2::<4096, 32>,
              benchmark_incremental_rw_io_uring_batch_alt2::<4096, 64>,
              benchmark_incremental_rw_io_uring_batch_alt2::<4096, 128>,
              benchmark_incremental_rw_io_uring_batch_alt2::<4096, 256>,
              benchmark_incremental_rw_io_uring_batch_alt2::<8192, 16>,
              benchmark_incremental_rw_io_uring_batch_alt2::<8192, 32>,
              benchmark_incremental_rw_io_uring_batch_alt2::<8192, 64>,
              benchmark_incremental_rw_io_uring_batch_alt2::<8192, 128>,
              benchmark_incremental_rw_io_uring_batch_alt2::<8192, 256>,
              benchmark_incremental_rw_io_uring_batch_alt2::<16384, 16>,
              benchmark_incremental_rw_io_uring_batch_alt2::<16384, 32>,
              benchmark_incremental_rw_io_uring_batch_alt2::<16384, 64>,
              benchmark_incremental_rw_io_uring_batch_alt2::<16384, 128>,
              benchmark_incremental_rw_io_uring_batch_alt2::<16384, 256>,
);
criterion_main!(benches);
