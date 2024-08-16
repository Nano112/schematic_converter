use criterion::{black_box, criterion_group, criterion_main, Criterion};
use schematic_converter::converters::{schem_to_schematic, schematic_to_schem};
use std::io::Cursor;

fn benchmark_schem_to_schematic(c: &mut Criterion) {
    let sample_schem = include_bytes!("../tests/test_schematics/sample.schem");

    c.bench_function("schem_to_schematic", |b| {
        b.iter(|| {
            let mut output = Vec::new();
            schem_to_schematic(Cursor::new(black_box(sample_schem)), &mut output).unwrap();
        })
    });
}

fn benchmark_schematic_to_schem(c: &mut Criterion) {
    let sample_schem = include_bytes!("../tests/test_schematics/sample.schem");
    let mut schematic_data = Vec::new();
    schem_to_schematic(Cursor::new(sample_schem), &mut schematic_data).unwrap();

    c.bench_function("schematic_to_schem", |b| {
        b.iter(|| {
            let mut output = Vec::new();
            schematic_to_schem(Cursor::new(black_box(&schematic_data)), &mut output).unwrap();
        })
    });
}

fn benchmark_roundtrip(c: &mut Criterion) {
    let sample_schem = include_bytes!("../tests/test_schematics/sample.schem");

    c.bench_function("roundtrip_conversion", |b| {
        b.iter(|| {
            let mut schematic_data = Vec::new();
            schem_to_schematic(Cursor::new(black_box(sample_schem)), &mut schematic_data).unwrap();

            let mut roundtrip_schem = Vec::new();
            schematic_to_schem(Cursor::new(&schematic_data), &mut roundtrip_schem).unwrap();
        })
    });
}

criterion_group!(benches, benchmark_schem_to_schematic, benchmark_schematic_to_schem, benchmark_roundtrip);
criterion_main!(benches);