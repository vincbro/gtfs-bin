//! Strategies for arbitrary networks, which the property tests share.

use flatnet_compiler::{
    Call, Network, RouteRecord, ServiceRecord, StopRecord, TransferRecord, TripRecord, Weekly,
};
use flatnet_core::models::{
    BoardingRule, Coordinate, Date, Direction, Distance, LocationType, Optional, RouteType, Time,
};
use proptest::{collection, option, prelude::*};

const LOCATION_TYPES: [LocationType; 5] = [
    LocationType::STOP_POINT,
    LocationType::STOP_AREA,
    LocationType::STATION_ENTRANCE,
    LocationType::GENERIC_NODE,
    LocationType::BOARDING_AREA,
];

/// The last day of the year 9999, which a network may hold.
pub(crate) const LAST_DAY: u32 = 2_932_896;

pub(crate) fn degrees() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(f64::NAN),
        Just(f64::INFINITY),
        Just(-1e9),
        Just(1000.0),
        Just(0.0),
        Just(59.3),
        -90.0..90.0,
        any::<f64>(),
    ]
}

pub(crate) fn date() -> impl Strategy<Value = Date> {
    prop_oneof![
        Just(Date(0)),
        Just(Date(LAST_DAY)),
        Just(Date(LAST_DAY + 1)),
        Just(Date(u32::MAX)),
        (17_000_u32..17_400).prop_map(Date),
        (0..LAST_DAY).prop_map(Date),
        any::<u32>().prop_map(Date),
    ]
}

pub(crate) fn time() -> impl Strategy<Value = Optional<Time>> {
    prop_oneof![
        Just(None),
        Just(Some(Time(u32::MAX))),
        Just(Some(Time(u32::MAX - 1))),
        (0_u32..90_000).prop_map(|seconds| Some(Time(seconds))),
    ]
    .prop_map(Optional::from)
}

pub(crate) fn stops() -> impl Strategy<Value = Vec<StopRecord>> {
    collection::vec(
        (
            option::of(0_usize..6),
            option::of(degrees()),
            option::of(degrees()),
            0..LOCATION_TYPES.len(),
        ),
        1..6,
    )
    .prop_map(|stops| {
        let count = stops.len();
        stops
            .into_iter()
            .enumerate()
            .map(
                |(index, (parent, latitude, longitude, location))| StopRecord {
                    id: format!("s{index}"),
                    code: None,
                    name: Some(format!("n{index}")),
                    description: None,
                    latitude,
                    longitude,
                    location_type: LOCATION_TYPES[location],
                    parent: parent.filter(|&parent| parent < count),
                },
            )
            .collect()
    })
}

pub(crate) fn services() -> impl Strategy<Value = Vec<ServiceRecord>> {
    let weekly = (any::<[bool; 7]>(), date(), date()).prop_map(|(days, start, end)| Weekly {
        days,
        start,
        end,
    });
    collection::vec(
        (
            option::of(weekly),
            collection::vec((date(), any::<bool>()), 0..3),
        ),
        1..3,
    )
    .prop_map(|services| {
        services
            .into_iter()
            .enumerate()
            .map(|(index, (weekly, exceptions))| ServiceRecord {
                id: format!("v{index}"),
                weekly,
                exceptions,
            })
            .collect()
    })
}

pub(crate) type CallParts = (
    usize,
    usize,
    u32,
    Optional<Time>,
    Optional<Time>,
    Option<u32>,
    Option<f32>,
);

pub(crate) fn calls() -> impl Strategy<Value = Vec<CallParts>> {
    collection::vec(
        (
            0_usize..4,
            0_usize..6,
            0_u32..4,
            time(),
            time(),
            option::of(0_u32..3),
            option::of(prop_oneof![
                Just(f32::NAN),
                Just(-1.0),
                Just(f32::INFINITY),
                0.0_f32..5000.0
            ]),
        ),
        0..24,
    )
}

pub(crate) fn network() -> impl Strategy<Value = Network> {
    let trips = collection::vec((0_usize..2, option::of(0_usize..3)), 1..4);
    let lines = collection::vec(collection::vec((degrees(), degrees()), 0..5), 0..3);
    let transfers = collection::vec(
        (
            0_usize..6,
            0_usize..6,
            option::of(0_usize..4),
            option::of(0_usize..4),
        ),
        0..4,
    );
    (stops(), services(), trips, calls(), lines, transfers).prop_map(
        |(stops, services, trips, calls, lines, transfers)| {
            let route = |index: usize, route_type| RouteRecord {
                id: format!("r{index}"),
                short_name: (index == 1).then(|| "x".to_owned()),
                long_name: None,
                description: None,
                route_type,
            };
            let lines: Vec<Vec<Coordinate>> = lines
                .into_iter()
                .map(|line| {
                    line.into_iter()
                        .map(|(latitude, longitude)| Coordinate::new(latitude, longitude))
                        .collect()
                })
                .collect();
            let trips: Vec<TripRecord> = trips
                .into_iter()
                .enumerate()
                .map(|(index, (route, service))| TripRecord {
                    id: format!("t{index}"),
                    route,
                    service: service.filter(|&service| service < services.len()),
                    headsign: None,
                    short_name: None,
                    line: (!lines.is_empty() && index % 2 == 0).then(|| index % lines.len()),
                    block: None,
                    direction: Direction::OUTBOUND,
                })
                .collect();
            let calls = calls
                .into_iter()
                .filter(|call| call.0 < trips.len() && call.1 < stops.len())
                .map(
                    |(trip, stop, sequence, arrival, departure, headsign, distance)| Call {
                        trip,
                        stop,
                        sequence,
                        arrival,
                        departure,
                        headsign: headsign.filter(|&headsign| headsign < 2),
                        distance: distance.map(Distance).into(),
                        pickup: BoardingRule::REGULAR,
                        drop_off: BoardingRule::REGULAR,
                    },
                )
                .collect();
            let transfers = transfers
                .into_iter()
                .filter(|transfer| transfer.0 < stops.len() && transfer.1 < stops.len())
                .map(|(from_stop, to_stop, from_trip, to_trip)| TransferRecord {
                    from_stop,
                    to_stop,
                    from_trip: from_trip.filter(|&trip| trip < trips.len()),
                    to_trip: to_trip.filter(|&trip| trip < trips.len()),
                    transfer_type: 1,
                    min_transfer_time: Some(30),
                })
                .collect();
            Network {
                stops,
                routes: vec![route(0, RouteType::BUS), route(1, RouteType::RAIL)],
                services,
                trips,
                lines,
                headsigns: vec!["a".to_owned(), "b".to_owned()],
                calls,
                transfers,
                ..Network::default()
            }
        },
    )
}
