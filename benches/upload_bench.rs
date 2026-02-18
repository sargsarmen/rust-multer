#![allow(missing_docs)]

use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use futures::stream;
use multigear::{MemoryStorage, Multer, MulterError};

fn benchmark_upload_parse(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let body = build_body(64 * 1024);

    c.bench_function("parse_and_store_64kb_file", |b| {
        b.to_async(&runtime).iter(|| async {
            let multer = Multer::new(MemoryStorage::new());
            let output = multer
                .parse_and_store(
                    "BOUND",
                    stream::iter([Ok::<Bytes, MulterError>(Bytes::from(body.clone()))]),
                )
                .await
                .expect("pipeline should succeed");
            assert_eq!(output.stored_files.len(), 1);
        });
    });
}

fn build_body(size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(size + 256);
    out.extend_from_slice(
        b"--BOUND\r\nContent-Disposition: form-data; name=\"upload\"; filename=\"bench.bin\"\r\n\r\n",
    );
    out.extend(std::iter::repeat(b'x').take(size));
    out.extend_from_slice(b"\r\n--BOUND--\r\n");
    out
}

criterion_group!(benches, benchmark_upload_parse);
criterion_main!(benches);

