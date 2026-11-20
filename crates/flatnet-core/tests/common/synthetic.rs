//! A feed generator for tests and benchmarks, so the code can be tried on a network far bigger
//! than the fixture without committing megabytes of text.
#![allow(dead_code)]

use std::{fmt::Write as _, fs, path::Path};

/// How big a generated network is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Size {
    pub stops: usize,
    pub routes: usize,
    pub trips: usize,
    pub stops_per_trip: usize,
}

impl Size {
    /// How many stop times a network of this size has.
    pub(crate) const fn stop_times(self) -> usize {
        self.trips * self.stops_per_trip
    }

    /// A few hundred trips, which compiles in milliseconds.
    pub(crate) const SMALL: Self = Self {
        stops: 300,
        routes: 30,
        trips: 600,
        stops_per_trip: 20,
    };

    /// About a hundred thousand stop times.
    pub(crate) const MEDIUM: Self = Self {
        stops: 1_500,
        routes: 120,
        trips: 6_000,
        stops_per_trip: 20,
    };

    /// About a million stop times, the size of a mid sized region.
    pub(crate) const LARGE: Self = Self {
        stops: 5_000,
        routes: 400,
        trips: 20_000,
        stops_per_trip: 50,
    };
}

/// A small deterministic generator, so a failing run can be repeated.
struct LinearCongruential(u64);

impl LinearCongruential {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, limit: usize) -> usize {
        let limit = u64::try_from(limit).expect("a limit fits in u64");
        usize::try_from(self.next() % limit).expect("a value below the limit fits in usize")
    }
}

/// A random offset of up to 0.3 degrees.
#[allow(
    clippy::cast_precision_loss,
    reason = "the value is below 300,000, far inside f64's exact range"
)]
fn microdegrees(random: &mut LinearCongruential) -> f64 {
    random.below(300_000) as f64 / 1_000_000.0
}

/// Writes a feed of the given size into `directory`. The same size always gives the same feed.
pub(crate) fn write_feed(directory: &Path, size: Size) {
    let mut random = LinearCongruential(7);
    let write = |name: &str, content: String| {
        fs::write(directory.join(name), content).expect("the directory is writable");
    };

    let mut stops =
        String::from("stop_id,stop_name,stop_lat,stop_lon,location_type,parent_station\n");
    for stop in 0..size.stops {
        let latitude = 59.2 + microdegrees(&mut random);
        let longitude = 17.9 + microdegrees(&mut random);
        writeln!(
            stops,
            "stop{stop},Stop number {},{latitude:.6},{longitude:.6},0,",
            stop / 3
        )
        .expect("writing to a string");
    }
    write("stops.txt", stops);

    let mut routes = String::from("route_id,route_short_name,route_long_name,route_type\n");
    for route in 0..size.routes {
        let route_type = [3, 1, 2, 4][route % 4];
        writeln!(routes, "route{route},{route},Route {route},{route_type}")
            .expect("writing to a string");
    }
    write("routes.txt", routes);

    let mut calendar = String::from(
        "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n",
    );
    for service in 0..10 {
        let weekend = service % 2;
        writeln!(
            calendar,
            "service{service},1,1,1,1,1,{weekend},{weekend},20240101,20241231"
        )
        .expect("writing to a string");
    }
    write("calendar.txt", calendar);
    write(
        "agency.txt",
        "agency_id,agency_name,agency_url,agency_timezone\nA,Agency,https://example.org,Europe/Stockholm\n".to_owned(),
    );

    let patterns: Vec<Vec<usize>> = (0..size.routes * 2)
        .map(|_| {
            (0..size.stops_per_trip)
                .map(|_| random.below(size.stops))
                .collect()
        })
        .collect();
    let mut trips = String::from("route_id,service_id,trip_id,trip_headsign,direction_id\n");
    let mut stop_times =
        String::from("trip_id,arrival_time,departure_time,stop_id,stop_sequence,pickup_type\n");
    for trip in 0..size.trips {
        let pattern = trip % (size.routes * 2);
        writeln!(
            trips,
            "route{},service{},trip{trip},Head {pattern},{}",
            pattern % size.routes,
            trip % 10,
            pattern % 2
        )
        .expect("writing to a string");
        let mut seconds = 5 * 3600 + (trip / 40) * 300;
        for (sequence, stop) in patterns[pattern].iter().enumerate() {
            seconds += 90;
            let (hours, minutes, rest) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
            writeln!(
                stop_times,
                "trip{trip},{hours:02}:{minutes:02}:{rest:02},{hours:02}:{minutes:02}:{rest:02},stop{stop},{sequence},0"
            )
            .expect("writing to a string");
        }
    }
    write("trips.txt", trips);
    write("stop_times.txt", stop_times);
}
