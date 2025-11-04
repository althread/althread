use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::process::Command;

fn run_test_file(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--package")
        .arg("althread-cli")
        .arg("compile")
        .arg(file)
        .output()?;

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err(format!("Failed to run {}", file).into());
    }

    Ok(())
}

fn create_benchmark(c: &mut Criterion, name: &str, file: &str) {
    let mut group = c.benchmark_group(format!("{}_group", name));
    group.measurement_time(std::time::Duration::from_secs(30));
    group.bench_function(BenchmarkId::new(name, "alt"), |b| {
        b.iter(|| run_test_file(file).expect(&format!("Failed to run {}", file)))
    });
    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    create_benchmark(c, "list_of_list", "../examples/list-of-list.alt");
    create_benchmark(c, "ring_election", "../examples/ring-election.alt");
    create_benchmark(c, "break", "../examples/test-break.alt");
    create_benchmark(c, "if_else", "../examples/test-if-else.alt");
    create_benchmark(c, "wait", "../examples/test-wait.alt");
    create_benchmark(c, "personal_mutual_exclusion", "../examples/peterson_mutual_exlusion.alt");
    create_benchmark(c, "atomic", "../examples/test-atomic.alt");
    create_benchmark(c, "channels", "../examples/test-channels.alt");
    create_benchmark(c, "list", "../examples/test-list.alt");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
