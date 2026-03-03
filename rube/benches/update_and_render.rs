use criterion::{Criterion, criterion_group, criterion_main};
use rube::{indirect::IndirectPass, march::MarchPass, scene::Scene};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let scale = 20;
    let width = 16 * scale;
    let height = 9 * scale;
    let scene = Scene::castle();
    c.bench_function("march_pass", |b| {
        b.iter_batched(
            || MarchPass::new(width, height),
            |mut march_pass| {
                rube::march::march_pass(
                    black_box(&scene),
                    black_box(&mut march_pass),
                    black_box(width),
                    black_box(height),
                )
            },
            criterion::BatchSize::LargeInput,
        )
    });
    let mut march_pass = MarchPass::new(width, height);
    rube::march::march_pass(&scene, &mut march_pass, width, height);
    c.bench_function("indirect_pass", |b| {
        b.iter_batched(
            || (IndirectPass::new(width, height), vec![0; width * height]),
            |(mut indirect_pass, mut pixels)| {
                rube::indirect::indirect_pass(
                    black_box(&scene),
                    black_box(&march_pass),
                    black_box(&mut indirect_pass),
                    black_box(&mut pixels),
                )
            },
            criterion::BatchSize::LargeInput,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

// STARTING
//
// march_pass              time:   [5.2652 ms 5.2841 ms 5.3071 ms]
// Found 7 outliers among 100 measurements (7.00%)
//   5 (5.00%) high mild
//   2 (2.00%) high severe
//
// indirect_pass           time:   [17.768 ms 17.795 ms 17.825 ms]
//  Found 4 outliers among 100 measurements (4.00%)
//  3 (3.00%) high mild
//  1 (1.00%) high severe

// PackedHitInfo
//
// march_pass              time:   [5.2664 ms 5.2914 ms 5.3223 ms]
// Found 4 outliers among 100 measurements (4.00%)
//   1 (1.00%) high mild
//   3 (3.00%) high severe
//
// indirect_pass           time:   [10.319 ms 10.355 ms 10.397 ms]
// Found 6 outliers among 100 measurements (6.00%)
//   6 (6.00%) high severe

// fxhash
//
// march_pass              time:   [5.2598 ms 5.2728 ms 5.2864 ms]
// Found 6 outliers among 100 measurements (6.00%)
//   6 (6.00%) high mild
//
// indirect_pass           time:   [9.9319 ms 9.9546 ms 9.9782 ms]
// Found 4 outliers among 100 measurements (4.00%)
//   4 (4.00%) high mild

// occlusion opt
// 
// march_pass              time:   [5.2674 ms 5.2848 ms 5.3042 ms]
// Found 5 outliers among 100 measurements (5.00%)
//   5 (5.00%) high mild
// 
// indirect_pass           time:   [8.9427 ms 8.9866 ms 9.0433 ms]
// Found 3 outliers among 100 measurements (3.00%)
//   1 (1.00%) high mild
//   2 (2.00%) high severe
