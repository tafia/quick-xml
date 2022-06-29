use criterion::criterion_main;

mod macrobenches;
mod microbenches;

criterion_main!(macrobenches::benches, microbenches::benches);
