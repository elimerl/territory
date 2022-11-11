use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use territory::world::World;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample-size-example");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs_f32(20.0));
    group.bench_function("100 cycles 512x512", |b| {
        b.iter(|| {
            let mut world = World::new(512, 512);

            for _ in 0..100 {
                world.update();
            }

            black_box(world);
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
