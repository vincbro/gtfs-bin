#![allow(clippy::all)]
#![allow(clippy::pedantic, clippy::restriction)]

use gtfs_bin::models::{BitMask, RouteType};

#[test]
fn test_route_types() {
    let route_type = RouteType::TRAM.join(RouteType::BUS).join(RouteType::SUBWAY);
    assert!(route_type.contains(RouteType::TRAM));
    assert!(route_type.contains(RouteType::BUS));
    assert!(route_type.contains(RouteType::SUBWAY));
    assert!(!route_type.contains(RouteType::RAIL));
    assert!(!route_type.contains(RouteType::TAXI));
}
