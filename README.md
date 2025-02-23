# Compio

A thread-per-core Rust runtime with IOCP/io_uring.
The name comes from "completion-based IO".
This crate is inspired by [monoio](https://github.com/bytedance/monoio/).

## Why not Tokio?

Tokio is a great generic-propose async runtime.
However, it is poll-based, and even uses [undocumented APIs](https://notgull.github.io/device-afd/) on Windows.
We would like some new high-level APIs to perform IOCP/io_uring.

Unlike `tokio-uring`, this runtime isn't Tokio-based.
This is mainly because that no public APIs to control IOCP in `mio`,
and `tokio` won't public APIs to control `mio` before `mio` reaches 1.0.

## Quick start
```rust,no_run
let buffer = compio::task::block_on(async {
    let file = compio::fs::OpenOptions::new().read(true).open("Cargo.toml").unwrap();
    let (read, buffer) = file.read_at(Vec::with_capacity(1024), 0).await;
    let read = read.unwrap();
    assert_eq!(read, buffer.len());
    String::from_utf8(buffer).unwrap()
});
println!("{}", buffer);
```
