use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rusty_reckoning::run;
use std::io;
use std::time::Duration;

struct NoopWriter;

impl io::Write for NoopWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Just return the length of input without actually writing
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn process_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    group.throughput(Throughput::Elements(1_000_000)); // 1M transactions in the input file
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(50);

    group.bench_function("process_10K_clients_1M_transactions", |b| {
        b.iter(|| {
            run("data/10K_clients.csv", NoopWriter).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, process_transactions);
criterion_main!(benches);
