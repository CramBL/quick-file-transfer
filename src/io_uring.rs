use std::{collections::VecDeque, fs::File, io, net::TcpStream, path::Path};

use rio::Rio;

pub fn incremental_rw_io_uring_simple<const BUFSIZE: usize>(
    socket: &TcpStream,
    file: &Path,
) -> anyhow::Result<u64> {
    let file = std::fs::OpenOptions::new().read(true).open(file)?;

    tracing::debug!("io uring");

    let buf = [0; BUFSIZE];
    let mut total_read = 0;
    let mut total_write = 0;

    let uring = rio::new()?;

    loop {
        let bytes_read = uring.read_at(&file, &buf, total_read).wait()?;
        if bytes_read == 0 {
            break; // EOF
        }
        total_read += bytes_read as u64;

        let mut write_offset = 0;
        while write_offset < bytes_read {
            let write_buf = &buf[write_offset..bytes_read];
            let written_bytes = uring.send(socket, &write_buf).wait()?;
            if written_bytes == 0 {
                // Consider handling the case where no bytes are written.
                // Might need to handle this depending on the socket's state and error handling.
                break;
            }
            write_offset += written_bytes;
        }
        total_write += write_offset as u64;

        debug_assert_eq!(
            bytes_read, write_offset,
            "Mismatch between bytes read/written, read={bytes_read}, written={write_offset}"
        );
    }

    debug_assert_eq!(
        total_read, total_write,
        "Mismatch between bytes read/written, total_read={total_read}, written={total_write}"
    );

    Ok(total_read)
}

pub fn incremental_rw_io_uring_batch<const BUFSIZE: usize, const BATCH_COUNT: usize>(
    socket: &TcpStream,
    file_path: &Path,
) -> anyhow::Result<u64> {
    let file = File::open(file_path)?;
    let buffers = [[0u8; BUFSIZE]; BATCH_COUNT];
    let mut total_read: u64 = 0;
    let mut total_write: u64 = 0;

    let uring = rio::new()?;
    let mut in_completions = Vec::with_capacity(BATCH_COUNT);

    'io_operation: loop {
        // Submit BATCH_COUNT reads
        for batch_idx in 0..buffers.len() {
            let file_offset = total_read + (batch_idx as u64 * BUFSIZE as u64);
            let read_future = uring.read_at(&file, &buffers[batch_idx], file_offset);
            in_completions.push(read_future);
        }

        // Collect results of read operations and send it to TCP socket
        for (idx, completion) in in_completions.drain(..).enumerate() {
            let bytes_read = completion.wait()?;
            if bytes_read == 0 {
                break 'io_operation; // EOF
            }
            total_read += bytes_read as u64;

            let buf = &buffers[idx][..bytes_read];
            total_write += write_at_socket(&uring, socket, buf, bytes_read)? as u64;
        }
    }

    // Assertions to ensure the correctness of read/write operations
    assert_eq!(
        total_read, total_write,
        "Total bytes read does not equal total bytes written"
    );

    Ok(total_read)
}

pub fn incremental_rw_io_uring_batch_alt<const BUFSIZE: usize, const BATCH_COUNT: usize>(
    socket: &TcpStream,
    file_path: &Path,
) -> anyhow::Result<u64> {
    let file = File::open(file_path)?;
    let buffers = [[0u8; BUFSIZE]; BATCH_COUNT];
    let mut total_read: u64 = 0;
    let mut total_write: u64 = 0;

    let config = rio::Config {
        depth: BATCH_COUNT,
        ..Default::default()
    };
    let uring = config.start()?;

    let mut in_completions = Vec::with_capacity(BATCH_COUNT);
    let mut write_buffer = Vec::with_capacity(BUFSIZE * BATCH_COUNT);

    for batch_idx in 0..buffers.len() {
        let file_offset = total_read + (batch_idx as u64 * BUFSIZE as u64);
        let read_future = uring.read_at(&file, &buffers[batch_idx], file_offset);
        in_completions.push(read_future);
    }

    let mut target_transfer_len = None;

    'io_operation: loop {
        // Collect results of read operations and send it to TCP socket
        for (idx, completion) in in_completions.drain(..).enumerate() {
            let bytes_read = completion.wait()?;
            total_read += bytes_read as u64;
            if bytes_read == 0 {
                target_transfer_len = Some(total_read); // EOF reached
                break;
            }
            write_buffer.extend_from_slice(&buffers[idx][..bytes_read]);
        }

        if target_transfer_len.is_none() {
            for batch_idx in 0..buffers.len() {
                let file_offset = total_read + (batch_idx as u64 * BUFSIZE as u64);
                let read_future = uring.read_at(&file, &buffers[batch_idx], file_offset);
                in_completions.push(read_future);
            }
            uring.submit_all();
        }

        // Write accumulated data in batches
        total_write += write_at_socket(&uring, socket, &write_buffer, write_buffer.len())? as u64;
        if target_transfer_len.is_some_and(|target| target == total_write) {
            break 'io_operation;
        }
        write_buffer.clear();
    }

    // Assertions to ensure the correctness of read/write operations
    assert_eq!(
        total_read, total_write,
        "Total bytes read does not equal total bytes written"
    );

    Ok(total_read)
}

pub fn incremental_rw_io_uring_batch_alt2<const BUFSIZE: usize, const BATCH_COUNT: usize>(
    socket: &TcpStream,
    file_path: &Path,
) -> anyhow::Result<u64> {
    let file = File::open(file_path)?;
    let buffers = [[0u8; BUFSIZE]; BATCH_COUNT];
    let mut total_read: u64 = 0;
    let mut total_write: u64 = 0;

    let config = rio::Config {
        depth: BATCH_COUNT,
        ..Default::default()
    };
    let uring = config.start()?;
    let mut read_futures = VecDeque::with_capacity(BATCH_COUNT);

    // Initialize read futures
    for batch_idx in 0..BATCH_COUNT {
        let file_offset = batch_idx as u64 * BUFSIZE as u64;
        let read_future = uring.read_at(&file, &buffers[batch_idx], file_offset);
        read_futures.push_back((batch_idx, read_future));
    }

    while let Some((idx, read_future)) = read_futures.pop_front() {
        let bytes_read = read_future.wait()?;
        if bytes_read == 0 {
            break; // EOF
        }
        total_read += bytes_read as u64;

        let buf = &buffers[idx][..bytes_read];
        total_write += write_at_socket(&uring, socket, buf, bytes_read)? as u64;

        debug_assert_eq!(total_read, total_write);

        // Re-issue the read for the same buffer
        let file_offset = total_read + (BUFSIZE as u64 * (BATCH_COUNT - 1) as u64);
        read_futures.push_back((idx, uring.read_at(&file, &buffers[idx], file_offset)));
    }

    assert_eq!(
        total_read, total_write,
        "Total bytes read does not equal total bytes written"
    );

    Ok(total_read)
}

#[inline(always)]
fn write_at_socket(
    uring: &Rio,
    socket: &TcpStream,
    buf: &[u8],
    write_count: usize,
) -> std::io::Result<usize> {
    let mut total_written = 0;
    while total_written < write_count {
        let tmp_write_buf = &buf[total_written..];
        let written_bytes = uring.send(socket, &tmp_write_buf).wait()?;
        if written_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Failed writing any bytes to the socket",
            ));
        }
        total_written += written_bytes;
    }
    Ok(total_written)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::{
        io::{self, Read, Write},
        net::TcpListener,
        path::PathBuf,
        thread,
    };

    use super::*;
    use temp_dir::TempDir;
    use testresult::TestResult;

    fn setup_test_environment(
        dir: &mut TempDir,
        data: &[u8],
    ) -> io::Result<(TcpStream, TcpStream, PathBuf)> {
        let file_path = dir.path().join("test_data.bin");

        // Write some test data to a temporary file
        let mut file = File::create(&file_path)?;
        file.write_all(data)?;

        // Setup a simple TCP server and client to simulate a socket connection
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let server_addr = listener.local_addr()?;
        let client = TcpStream::connect(server_addr)?;
        let (server, _) = listener.accept()?;

        Ok((client, server, file_path))
    }

    #[test]
    fn test_incremental_rw_io_uring_batched() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;

        let data = (0..101024).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_alt() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..123456).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_small_file() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..100).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_large_file() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..100 * 1024 * 1024)
            .map(|i| (i % 256) as u8)
            .collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Run the receiving part in a separate thread, this is necessary when the file size is large as the
        //  TCP buffers will fill up and so the client will block before we reach the code where the server is receiving
        let receiver_thread = thread::spawn(move || {
            let mut received_data = Vec::new();
            server_socket
                .read_to_end(&mut received_data)
                .expect("Failed to read data from server socket");
            received_data // Return the data directly from the thread
        });

        // Perform the operation
        let result = incremental_rw_io_uring_batch::<1024, 10>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Retrieve data from the receiver thread
        let received_data = receiver_thread
            .join()
            .expect("Receiver thread has panicked");
        assert_eq!(received_data, data);
        Ok(())
    }

    // For batched tcp

    #[test]
    fn test_incremental_rw_io_uring_batched_tcp() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;

        let data = (0..101024).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_tcp_alt() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..123456).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_tcp_small_file() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..100).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_alt_large_file() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..100 * 1024 * 1024)
            .map(|i| (i % 256) as u8)
            .collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Run the receiving part in a separate thread, this is necessary when the file size is large as the
        //  TCP buffers will fill up and so the client will block before we reach the code where the server is receiving
        let receiver_thread = thread::spawn(move || {
            let mut received_data = Vec::new();
            server_socket
                .read_to_end(&mut received_data)
                .expect("Failed to read data from server socket");
            received_data // Return the data directly from the thread
        });

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt::<1024, 16>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Retrieve data from the receiver thread
        let received_data = receiver_thread
            .join()
            .expect("Receiver thread has panicked");
        assert_eq!(received_data, data);
        Ok(())
    }

    // For alt2

    #[test]
    fn test_incremental_rw_io_uring_batched_alt2_small_file() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..100).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt2::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_alt2_small_file_2() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..500).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt2::<128, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    fn find_first_difference(v1: &[u8], v2: &[u8]) -> Option<usize> {
        let min_len = std::cmp::min(v1.len(), v2.len());
        (0..min_len).find(|&i| v1[i] != v2[i])
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_alt2() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;

        let data = (0..101024).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt2::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");

        assert_eq!(received_data.len(), data.len());
        let diff_idx = find_first_difference(&received_data, &data);
        println!("{diff_idx:?}");

        if let Some(idx) = diff_idx {
            assert_eq!(received_data[idx..idx + 100], data[idx..idx + 100]);
        }

        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_alt_alt2() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..123456).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt2::<1024, 2>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Verify data received on the server side matches the data sent
        let mut received_data = Vec::new();
        server_socket
            .read_to_end(&mut received_data)
            .expect("Failed to read data from server socket");
        assert_eq!(received_data, data);
        Ok(())
    }

    #[test]
    fn test_incremental_rw_io_uring_batched_alt2_large_file() -> TestResult {
        // Create a temporary directory
        let mut dir = TempDir::new()?;
        let data = (0..100 * 1024 * 1024)
            .map(|i| (i % 256) as u8)
            .collect::<Vec<u8>>();
        let (client_socket, mut server_socket, file_path) =
            setup_test_environment(&mut dir, &data).expect("Failed to set up test environment");

        // Run the receiving part in a separate thread, this is necessary when the file size is large as the
        //  TCP buffers will fill up and so the client will block before we reach the code where the server is receiving
        let receiver_thread = thread::spawn(move || {
            let mut received_data = Vec::new();
            server_socket
                .read_to_end(&mut received_data)
                .expect("Failed to read data from server socket");
            received_data // Return the data directly from the thread
        });

        // Perform the operation
        let result = incremental_rw_io_uring_batch_alt2::<1024, 16>(&client_socket, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        drop(client_socket);

        // Retrieve data from the receiver thread
        let received_data = receiver_thread
            .join()
            .expect("Receiver thread has panicked");
        assert_eq!(received_data, data);
        Ok(())
    }
}
