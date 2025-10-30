use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nx_syntax::parse_str;

fn generate_nx_source(lines: usize) -> String {
    let mut source = String::new();

    // Add function definitions
    for i in 0..lines / 10 {
        source.push_str(&format!(
            "fn calculate{}(x: int, y: int): int {{\n  return x + y * {};\n}}\n\n",
            i, i
        ));
    }

    // Add elements with interpolation
    for i in 0..lines / 5 {
        source.push_str(&format!(
            "<div id=\"container{}\">\n  <p>Value: {{calculate{}(10, 20)}}</p>\n</div>\n\n",
            i,
            i % (lines / 10)
        ));
    }

    source
}

fn parse_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for lines in [100, 1000, 10000].iter() {
        let source = generate_nx_source(*lines);
        let bytes = source.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("lines", lines), &source, |b, source| {
            b.iter(|| {
                let result = parse_str(black_box(source), black_box("benchmark.nx"));
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, parse_benchmark);
criterion_main!(benches);
