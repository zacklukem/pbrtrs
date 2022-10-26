use criterion::criterion_main;

mod util;

criterion_main! {
    util::benches
}
