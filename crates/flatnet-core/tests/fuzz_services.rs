//! Property tests that compare which days a service runs with a plain calendar.

mod common;

use flatnet_compiler::{
    Compiler, Network, RouteRecord, ServiceRecord, StopRecord, TripRecord, Weekly,
};
use flatnet_core::{
    consumer::Consumer,
    models::{Date, Direction, LocationType, RouteType, Sentinel, ServiceIndex},
};
use proptest::{collection, option, prelude::*};

/// Monday is 0, from the calendar day by Sakamoto's method, which shares nothing with
/// `Date::day_of_week`.
fn weekday(date: Date) -> usize {
    const OFFSETS: [u32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let (year, month, day) = date.to_year_month_day().expect("a real day");
    let year = if month < 3 { year - 1 } else { year };
    let from_sunday =
        (year + year / 4 - year / 100 + year / 400 + OFFSETS[month as usize - 1] + day) % 7;
    ((from_sunday + 6) % 7) as usize
}

/// Whether the service runs on `day`, the way the GTFS spec says: the weekly pattern between its
/// dates, then each exception in order.
fn runs(record: &ServiceRecord, day: u32) -> bool {
    let mut runs = record.weekly.is_some_and(|weekly| {
        (weekly.start.0..=weekly.end.0).contains(&day) && weekly.days[weekday(Date(day))]
    });
    for &(date, added) in &record.exceptions {
        if date.0 == day {
            runs = added;
        }
    }
    runs
}

fn service() -> impl Strategy<Value = ServiceRecord> {
    let origin = prop_oneof![
        Just(0_u32),
        Just(59),
        Just(2_932_000),
        17_000_u32..17_400,
        (0_u32..2_932_000),
    ];
    (
        origin,
        option::of((any::<[bool; 7]>(), 0_u32..120)),
        collection::vec((0_u32..160, any::<bool>()), 0..6),
    )
        .prop_map(|(origin, weekly, exceptions)| ServiceRecord {
            id: "s".to_owned(),
            weekly: weekly.map(|(days, length)| Weekly {
                days,
                start: Date(origin + 20),
                end: Date(origin + 20 + length),
            }),
            // Some fall before the week, so they stretch the service's first day.
            exceptions: exceptions
                .into_iter()
                .map(|(offset, added)| (Date(origin + offset), added))
                .collect(),
        })
}

fn network(services: Vec<ServiceRecord>) -> Network {
    Network {
        stops: vec![StopRecord {
            id: "a".to_owned(),
            code: None,
            name: None,
            description: None,
            latitude: Some(59.0),
            longitude: Some(18.0),
            location_type: LocationType::STOP_POINT,
            parent: None,
        }],
        routes: vec![RouteRecord {
            id: "r".to_owned(),
            short_name: None,
            long_name: None,
            description: None,
            route_type: RouteType::BUS,
        }],
        trips: (0..services.len())
            .map(|index| TripRecord {
                id: format!("t{index}"),
                route: 0,
                service: Some(index),
                headsign: None,
                short_name: None,
                line: None,
                block: None,
                direction: Direction::OUTBOUND,
            })
            .collect(),
        services,
        ..Network::default()
    }
}

proptest! {

    #[test]
    fn a_service_runs_on_the_days_a_plain_calendar_says(
        services in collection::vec(service(), 1..4),
    ) {
        let services: Vec<ServiceRecord> = services
            .into_iter()
            .enumerate()
            .map(|(index, record)| ServiceRecord { id: format!("s{index}"), ..record })
            .collect();
        let Ok(bytes) = Compiler::new().compile(network(services.clone())) else {
            // A service with no days, or one that runs for centuries, is refused.
            return Ok(());
        };
        let (_file, mmap) = common::map(&bytes).expect("the file maps");
        let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");

        for (position, record) in services.iter().enumerate() {
            let service = consumer.service_by_id(&record.id).expect("every service is kept");
            let first = service.start_date.0.saturating_sub(3);
            for day in first..=service.end_date.0 + 3 {
                prop_assert_eq!(
                    consumer.is_service_active(service.index, Date(day)),
                    runs(record, day),
                    "service {} on day {} ({})", position, day, Date(day)
                );
            }
        }
        prop_assert!(
            !consumer.is_service_active(ServiceIndex::NONE, Date(17_167)),
            "a trip with no service never runs"
        );
    }
}
