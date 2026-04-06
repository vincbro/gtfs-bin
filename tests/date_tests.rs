use gtfs_bin::models::{Date, WeekdaySet};

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
    let weekday = WeekdaySet::new().with_monday(true);
    assert!(weekday.monday());
    assert!(!weekday.tuesday());
    let weekday = WeekdaySet::new().with_friday(true);
    assert!(weekday.friday());
    assert!(!weekday.thursday());
    assert!(!weekday.saturday());
}

#[test]
fn test_weekday_max() {
    let weekday = WeekdaySet(u8::MAX);

    assert!(weekday.monday());
    assert!(weekday.tuesday());
    assert!(weekday.wednesday());
    assert!(weekday.thursday());
    assert!(weekday.friday());
    assert!(weekday.saturday());
    assert!(weekday.sunday());
}
