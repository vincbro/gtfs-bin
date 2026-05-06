#![allow(clippy::all)]
#![allow(clippy::pedantic, clippy::restriction)]

use gtfs_bin::models::{BitMask, Date, Weekday};

#[test]
fn test_unix_epoch() {
    let date = Date(0);
    assert_eq!(date.to_string(), "1970-01-01");
}

#[test]
fn test_future_date() {
    let date = Date(20_533);
    assert_eq!(date.to_string(), "2026-03-21");
}

#[test]
fn test_leap_year_day() {
    // February 29, 2024 is 19,782 days after epoch
    let date = Date(19_782);
    assert_eq!(date.to_string(), "2024-02-29");
}

#[test]
fn test_dates() {
    let weekday = Weekday::MONDAY;
    assert!(weekday.contains(Weekday::MONDAY));
    assert!(!weekday.contains(Weekday::TUESDAY));
    let weekday = Weekday::FRIDAY;
    assert!(weekday.contains(Weekday::FRIDAY));
    assert!(!weekday.contains(Weekday::THURSDAY));
    assert!(!weekday.contains(Weekday::SATURDAY));
}

#[test]
fn test_weekday_max() {
    let all = Weekday::MONDAY
        .join(Weekday::TUESDAY)
        .join(Weekday::WEDNESDAY)
        .join(Weekday::THURSDAY)
        .join(Weekday::FRIDAY)
        .join(Weekday::SATURDAY)
        .join(Weekday::SUNDAY);
    let max = Weekday(u8::MAX);
    assert!(max.contains(all));
}
