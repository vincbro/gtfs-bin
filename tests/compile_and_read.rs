use std::{fs::File, path::PathBuf};

use gtfs_bin::{compiler::Compiler, consumer::Consumer};
use memmap2::Mmap;

#[test]
fn test_compile_and_consume_micro_feed() {
    let mut input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    input_path.push("tests/fixtures/gtfs.zip");

    let compiled_bytes = Compiler::new(input_path)
        .compile()
        .expect("Failed to compile GTFS");

    let temp_file_path = "tests/fixtures/temp_output.gtfs";
    std::fs::write(temp_file_path, &compiled_bytes).unwrap();

    let file = File::open(temp_file_path).unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };

    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.trips.len(), 1);
    assert_eq!(consumer.stops.len(), 5);
    assert_eq!(consumer.stop_times.len(), 2);
    assert_eq!(consumer.routes.len(), 1);

    std::fs::remove_file(temp_file_path).unwrap();
}
