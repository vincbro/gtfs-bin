//! The feed reader: formats it accepts and the errors it gives for the rest.

mod common;

use std::{fs, io::Write, path::Path};

use gtfs_bin::{
    compiler::{Compiler, Error},
    consumer::Consumer,
};
use zip::{ZipWriter, write::SimpleFileOptions};

const STOPS: &str =
    "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,59.30,18.00\nB,Beta,59.31,18.01\n";
const ROUTES: &str = "route_id,route_short_name,route_type\nR,1,3\n";
const TRIPS: &str = "route_id,service_id,trip_id\nR,S,T\n";
const STOP_TIMES: &str = "trip_id,stop_id,stop_sequence,arrival_time,departure_time\n\
    T,A,1,08:00:00,08:00:00\nT,B,2,08:10:00,08:10:00\n";
const CALENDAR: &str = "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n\
    S,1,1,1,1,1,0,0,20240101,20240131\n";

fn write_feed(directory: &Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        fs::write(directory.join(name), content).expect("the temp directory is writable");
    }
}

fn minimal() -> Vec<(&'static str, &'static str)> {
    vec![
        ("stops.txt", STOPS),
        ("routes.txt", ROUTES),
        ("trips.txt", TRIPS),
        ("stop_times.txt", STOP_TIMES),
        ("calendar.txt", CALENDAR),
    ]
}

#[allow(clippy::unwrap_in_result, reason = "a test helper that fails the test")]
fn compile(files: &[(&str, &str)]) -> Result<Vec<u8>, Error> {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write_feed(directory.path(), files);
    Compiler::new(directory.path()).compile()
}

fn with_file<'a>(
    mut files: Vec<(&'a str, &'a str)>,
    name: &'a str,
    content: &'a str,
) -> Vec<(&'a str, &'a str)> {
    files.retain(|(existing, _)| *existing != name);
    files.push((name, content));
    files
}

fn open(bytes: &[u8], check: impl FnOnce(&Consumer)) {
    let (_file, mmap) = common::map(bytes).expect("a compiled file maps");
    check(&Consumer::new(&mmap).expect("the compiler wrote a valid file"));
}

#[test]
fn a_minimal_feed_compiles() {
    let bytes = compile(&minimal()).expect("a minimal feed is valid");

    open(&bytes, |consumer| {
        assert_eq!(consumer.stops().len(), 2, "two stops");
        assert_eq!(consumer.trips().len(), 1, "one trip");
        assert_eq!(
            consumer
                .stop_times_by_trip(consumer.trip_by_id("T").expect("trip T").index)
                .len(),
            2,
            "both stop times"
        );
    });
}

#[test]
fn a_zip_gives_the_same_bytes_as_its_directory() {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    let archive_path = directory.path().join("feed.zip");
    let mut archive = ZipWriter::new(fs::File::create(&archive_path).expect("the zip is writable"));
    for entry in fs::read_dir(common::fixture_path()).expect("the fixture directory exists") {
        let entry = entry.expect("a readable entry");
        archive
            .start_file(
                entry.file_name().to_string_lossy(),
                SimpleFileOptions::default(),
            )
            .expect("an entry can start");
        archive
            .write_all(&fs::read(entry.path()).expect("a readable file"))
            .expect("the zip is writable");
    }
    archive.finish().expect("the zip can finish");

    let from_zip = Compiler::new(&archive_path)
        .compile()
        .expect("the zip is a feed");

    assert_eq!(
        from_zip,
        common::compile_fixture(),
        "the container must not change the output"
    );
}

#[test]
fn files_inside_a_folder_of_the_zip_are_found() {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    let archive_path = directory.path().join("feed.zip");
    let mut archive = ZipWriter::new(fs::File::create(&archive_path).expect("the zip is writable"));
    for (name, content) in minimal() {
        archive
            .start_file(format!("feed/{name}"), SimpleFileOptions::default())
            .expect("an entry can start");
        archive
            .write_all(content.as_bytes())
            .expect("the zip is writable");
    }
    archive.finish().expect("the zip can finish");

    let result = Compiler::new(&archive_path).compile();

    assert!(
        result.is_ok(),
        "publishers often zip the folder itself: {result:?}"
    );
}

#[test]
fn a_file_that_is_not_a_zip_is_an_error() {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    let path = directory.path().join("feed.zip");
    fs::write(&path, b"not a zip").expect("the temp directory is writable");

    assert!(
        matches!(Compiler::new(&path).compile(), Err(Error::Zip(_))),
        "the archive is rejected"
    );
}

#[test]
fn a_missing_path_is_an_error() {
    assert!(
        matches!(
            Compiler::new(Path::new("/definitely/not/here.zip")).compile(),
            Err(Error::Io { .. })
        ),
        "there is nothing to read"
    );
}

#[test]
fn a_byte_order_mark_and_windows_line_endings_are_accepted() {
    let stops = format!("\u{feff}{}", STOPS.replace('\n', "\r\n"));
    let trips = TRIPS.replace('\n', "\r\n");

    let result = compile(&with_file(
        with_file(minimal(), "stops.txt", &stops),
        "trips.txt",
        &trips,
    ));

    assert!(
        result.is_ok(),
        "both are common in published feeds: {result:?}"
    );
}

#[test]
fn quoted_fields_may_hold_commas_and_new_lines() {
    let stops = "stop_id,stop_name,stop_lat,stop_lon\n\
        A,\"Alpha, Central\",59.30,18.00\nB,\"Beta\nPlatform 2\",59.31,18.01\n";

    let bytes = compile(&with_file(minimal(), "stops.txt", stops)).expect("a valid feed");

    open(&bytes, |consumer| {
        let name = |id: &str| {
            let stop = consumer.stop_by_id(id).expect("the stop exists");
            consumer.string(stop.name.as_option().expect("a name"))
        };
        assert_eq!(name("A"), "Alpha, Central", "the comma stays in the name");
        assert_eq!(name("B"), "Beta\nPlatform 2", "so does the new line");
    });
}

#[test]
fn columns_may_come_in_any_order_with_extra_ones() {
    let stops = "wheelchair,stop_lon,stop_name,stop_id,stop_lat,unknown\n\
        1,18.00,Alpha,A,59.30,x\n0,18.01,Beta,B,59.31,y\n";

    let bytes = compile(&with_file(minimal(), "stops.txt", stops)).expect("a valid feed");

    open(&bytes, |consumer| {
        let stop = consumer.stop_by_id("B").expect("the stop exists");
        assert_eq!(
            consumer.string(stop.name.as_option().expect("a name")),
            "Beta",
            "found by header"
        );
    });
}

#[test]
fn only_calendar_dates_is_a_valid_calendar() {
    let calendar_dates = "service_id,date,exception_type\nS,20240102,1\nS,20240103,1\n";
    let mut files = minimal();
    files.retain(|(name, _)| *name != "calendar.txt");

    let bytes =
        compile(&with_file(files, "calendar_dates.txt", calendar_dates)).expect("a valid feed");

    open(&bytes, |consumer| {
        let service = consumer.service_by_id("S").expect("the service exists");
        let jan =
            |day| gtfs_bin::models::Date::parse_gtfs(&format!("202401{day:02}")).expect("a date");
        assert!(
            consumer.is_service_active(service.index, jan(2)),
            "added on the 2nd"
        );
        assert!(
            consumer.is_service_active(service.index, jan(3)),
            "added on the 3rd"
        );
    });
}

#[test]
fn a_feed_without_any_calendar_is_an_error() {
    let mut files = minimal();
    files.retain(|(name, _)| *name != "calendar.txt");

    assert!(
        matches!(compile(&files), Err(Error::MissingFile(_))),
        "service days can't be known"
    );
}

#[test]
fn a_missing_required_column_names_the_file_and_column() {
    let stops = "stop_name,stop_lat,stop_lon\nAlpha,59.30,18.00\n";

    let error = compile(&with_file(minimal(), "stops.txt", stops)).expect_err("stop_id is needed");

    assert!(
        matches!(
            error,
            Error::MissingColumn {
                file: "stops.txt",
                column: "stop_id"
            }
        ),
        "got {error:?}"
    );
}

#[test]
fn a_bad_value_names_the_file_line_and_column() {
    let stops = "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,59.30,18.00\nB,Beta,north,18.01\n";

    let error = compile(&with_file(minimal(), "stops.txt", stops)).expect_err("a bad latitude");

    match error {
        Error::InvalidValue {
            file,
            line,
            column,
            value,
        } => {
            assert_eq!(file, "stops.txt", "the file is named");
            assert_eq!(line, 3, "the third line holds the bad row");
            assert_eq!(column, "stop_lat", "the column is named");
            assert_eq!(value, "north", "and the value");
        }
        other => panic!("expected an invalid value, got {other:?}"),
    }
}

#[test]
fn impossible_dates_and_times_are_errors() {
    let bad_date = CALENDAR.replace("20240131", "20240230");
    let bad_time = STOP_TIMES.replace("08:10:00", "08:xx:00");

    assert!(
        matches!(
            compile(&with_file(minimal(), "calendar.txt", &bad_date)),
            Err(Error::InvalidValue {
                column: "end_date",
                ..
            })
        ),
        "February has no 30th"
    );
    assert!(
        matches!(
            compile(&with_file(minimal(), "stop_times.txt", &bad_time)),
            Err(Error::InvalidValue { .. })
        ),
        "a time must be numbers"
    );
}

#[test]
fn non_finite_coordinates_are_errors() {
    let stops = "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,NaN,18.00\n";

    assert!(
        matches!(
            compile(&with_file(minimal(), "stops.txt", stops)),
            Err(Error::InvalidValue {
                column: "stop_lat",
                ..
            })
        ),
        "NaN is not a place"
    );
}

#[test]
fn a_service_that_ends_before_it_starts_is_an_error() {
    let reversed = CALENDAR.replace("20240101,20240131", "20240131,20240101");

    assert!(
        matches!(
            compile(&with_file(minimal(), "calendar.txt", &reversed)),
            Err(Error::ReversedServiceRange { .. })
        ),
        "the range is backwards"
    );
}

#[test]
fn a_service_that_runs_for_centuries_is_an_error() {
    let long = CALENDAR.replace("20240131", "99991231");

    assert!(
        matches!(
            compile(&with_file(minimal(), "calendar.txt", &long)),
            Err(Error::ServiceRangeTooLong { .. })
        ),
        "a bad end date must not cost hundreds of megabytes"
    );
}

#[test]
fn the_first_of_a_repeated_id_wins() {
    let stops = "stop_id,stop_name,stop_lat,stop_lon\nA,First,59.30,18.00\nA,Second,59.31,18.01\nB,Beta,59.32,18.02\n";

    let bytes = compile(&with_file(minimal(), "stops.txt", stops)).expect("a valid feed");

    open(&bytes, |consumer| {
        assert_eq!(consumer.stops().len(), 2, "the repeat is dropped");
        let stop = consumer.stop_by_id("A").expect("A exists");
        assert_eq!(
            consumer.string(stop.name.as_option().expect("a name")),
            "First",
            "the first row stays"
        );
    });
}

#[test]
fn rows_that_point_at_unknown_things_are_skipped() {
    let trips = "route_id,service_id,trip_id\nR,S,T\nGHOST,S,U\n";
    let stop_times = "trip_id,stop_id,stop_sequence,arrival_time,departure_time\n\
        T,A,1,08:00:00,08:00:00\nT,GHOST,2,08:05:00,08:05:00\nU,A,1,09:00:00,09:00:00\nGHOST,A,1,09:00:00,09:00:00\n";

    let bytes = compile(&with_file(
        with_file(minimal(), "trips.txt", trips),
        "stop_times.txt",
        stop_times,
    ))
    .expect("a valid feed");

    open(&bytes, |consumer| {
        assert_eq!(
            consumer.trips().len(),
            1,
            "the trip on an unknown route is dropped"
        );
        assert_eq!(
            consumer.stop_times().len(),
            1,
            "only the fully known stop time stays"
        );
    });
}

#[test]
fn stop_times_without_times_are_kept() {
    let stop_times = "trip_id,stop_id,stop_sequence,arrival_time,departure_time\n\
        T,A,1,08:00:00,08:00:00\nT,B,2,,\n";

    let bytes = compile(&with_file(minimal(), "stop_times.txt", stop_times)).expect("a valid feed");

    open(&bytes, |consumer| {
        let stop_times = consumer.stop_times();
        assert!(
            stop_times[1].arrival_time.is_none(),
            "an empty time is absent, not zero"
        );
    });
}

#[test]
fn extended_route_types_are_grouped() {
    let routes = "route_id,route_short_name,route_type\nR,1,714\n";

    let bytes = compile(&with_file(minimal(), "routes.txt", routes)).expect("a valid feed");

    open(&bytes, |consumer| {
        assert_eq!(
            consumer.routes()[0].route_type,
            gtfs_bin::models::RouteType::BUS,
            "714 is a bus service"
        );
    });
}
