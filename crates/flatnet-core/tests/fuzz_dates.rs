//! Property tests for the date and time parsing, and for decoding realtime bytes.

mod common;

use flatnet_core::{
    consumer::Consumer,
    models::{Date, Delay, Time},
    rt::{self, Realtime, RealtimeBuilder},
};
use proptest::{collection, prelude::*};

/// The last day of the year 9999.
const LAST_DAY: u32 = 2_932_896;

#[test]
fn every_day_up_to_year_9999_round_trips_and_follows_the_one_before() {
    let mut previous = (1969, 12, 31);
    for day in 0..=LAST_DAY {
        let date = Date(day);
        let (year, month, of_month) = date.to_year_month_day().expect("a day in range");
        assert_eq!(
            Date::from_year_month_day(year, month, of_month),
            Some(date),
            "{year}-{month}-{of_month} gives day {day} back"
        );
        assert!(
            (year, month, of_month) > previous,
            "day {day} comes after the day before"
        );
        previous = (year, month, of_month);
        assert_eq!(
            usize::from(date.day_of_week()),
            ((day + 3) % 7) as usize,
            "weekdays cycle through the week from a Thursday"
        );
    }
    assert_eq!(previous, (9999, 12, 31), "the last day is the last day");
    assert_eq!(
        Date(LAST_DAY + 1).to_year_month_day(),
        None,
        "and nothing follows"
    );
}

proptest! {

    #[test]
    fn any_text_parses_to_a_date_that_prints_the_same_text_or_to_nothing(text in "[0-9a-z -]{0,10}") {
        if let Some(date) = Date::parse_gtfs(&text) {
            prop_assert_eq!(date.to_string().replace('-', ""), text, "a parsed date prints as it came");
        }
    }

    #[test]
    fn any_day_formats_and_parses_back(day in 0..=LAST_DAY) {
        let date = Date(day);
        let text = date.to_string().replace('-', "");
        prop_assert_eq!(Date::parse_gtfs(&text), Some(date));
    }

    #[test]
    fn any_text_parses_to_a_time_or_to_nothing(text in "[0-9:a-z -]{0,14}") {
        let _ = Time::from_hms(&text);
    }

    #[test]
    fn a_written_time_reads_back_unless_it_overflows(
        hours in 0_u32..1_000_000,
        minutes in 0_u32..100,
        seconds in 0_u32..100,
    ) {
        let parsed = Time::from_hms(&format!("{hours}:{minutes:02}:{seconds:02}"));
        let total = hours * 3600 + minutes * 60 + seconds;
        prop_assert_eq!(parsed, Some(Time(total)));
    }

    #[test]
    fn any_difference_makes_a_delay_without_panicking(seconds in any::<i64>()) {
        let _ = Delay::from(seconds);
    }

    #[test]
    fn arbitrary_bytes_decode_to_an_error_or_a_feed_the_builder_takes(
        bytes in collection::vec(any::<u8>(), 0..256),
    ) {
        let Ok(message) = rt::decode_feed(&bytes) else {
            return Ok(());
        };
        let compiled = common::compile_fixture();
        let (_file, mmap) = common::map(&compiled).expect("the fixture maps");
        let consumer = Consumer::new(&mmap).expect("the fixture is valid");
        let mut realtime = Realtime::new(&consumer);

        let _ = RealtimeBuilder::new(&consumer).apply([message].into_iter(), &mut realtime);

        prop_assert_eq!(realtime.trip_status().len(), consumer.trips().len());
    }
}
