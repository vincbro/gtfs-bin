use std::collections::HashMap;

use bitvec::{order::Msb0, vec::BitVec};
use gtfs_bin_core::models::{
    BitMask, Date, Sentinel, Service, ServiceBinarySlice, ServiceIdSlice, ServiceIndex, Slice,
    Weekday,
};

use crate::{
    Error, Result,
    records::{RawCalendar, RawCalendarDate},
    table::SkippedRow,
    tables,
};

/// The longest a service may run, about 100 years. It bounds the memory a bad date can cost, since
/// every service stores a bit for each of its days.
const MAX_SERVICE_DAYS: u32 = 36_600;

const WEEKDAYS: [Weekday; 7] = [
    Weekday::MONDAY,
    Weekday::TUESDAY,
    Weekday::WEDNESDAY,
    Weekday::THURSDAY,
    Weekday::FRIDAY,
    Weekday::SATURDAY,
    Weekday::SUNDAY,
];

/// The services, the map from a service id to its index, and the bits of days each service runs.
pub(super) struct BuiltServices {
    pub services: Vec<Service>,
    pub id_map: HashMap<String, ServiceIndex>,
    pub active_mask: BitVec<u8, Msb0>,
    /// The calendar rows lenient mode left out.
    pub skipped: Vec<SkippedRow>,
}

pub(super) fn build_services(
    raw_calendar: &[RawCalendar],
    raw_calendar_dates: &[RawCalendarDate],
    lenient: bool,
) -> Result<BuiltServices> {
    let mut id_map: HashMap<String, ServiceIndex> = HashMap::with_capacity(raw_calendar.len());
    // The dates the weekly pattern applies to, before calendar dates stretch a service's range.
    let mut base_bounds: Vec<(Date, Date)> = Vec::with_capacity(raw_calendar.len());
    let mut services: Vec<Service> = Vec::with_capacity(raw_calendar.len());

    // A GTFS set can have only a calendar, only calendar dates, or both. The calendar goes first,
    // and the calendar dates are folded into it afterwards.
    let mut skipped = Vec::new();
    for calendar in raw_calendar {
        if calendar.end_date < calendar.start_date {
            let error = Error::ReversedServiceRange {
                service_id: calendar.id.clone(),
            };
            if !lenient {
                return Err(error);
            }
            skipped.push(SkippedRow {
                file: "calendar.txt",
                message: error.to_string(),
            });
            continue;
        }
        let index = ServiceIndex::from(services.len());
        id_map.insert(calendar.id.clone(), index);
        base_bounds.push((calendar.start_date, calendar.end_date));
        services.push(Service::new(
            ServiceIdSlice::NONE,
            index,
            ServiceBinarySlice::NONE,
            calendar.start_date,
            calendar.end_date,
            weekdays(calendar),
        ));
    }

    for calendar_date in raw_calendar_dates {
        let date = calendar_date.date;
        if let Some(index) = id_map.get(&calendar_date.service_id) {
            let service = &mut services[index.as_usize()];
            service.start_date = service.start_date.min(date);
            service.end_date = service.end_date.max(date);
            continue;
        }

        // A service that only has calendar dates never runs on a weekly pattern, so its base range
        // is empty.
        let index = ServiceIndex::from(services.len());
        id_map.insert(calendar_date.service_id.clone(), index);
        base_bounds.push((Date(u32::MAX), Date(u32::MIN)));
        services.push(Service::new(
            ServiceIdSlice::NONE,
            index,
            ServiceBinarySlice::NONE,
            date,
            date,
            Weekday::default(),
        ));
    }

    let active_mask = build_active_mask(
        &mut services,
        &base_bounds,
        raw_calendar_dates,
        &id_map,
        &raw_service_ids(&id_map),
    )?;
    Ok(BuiltServices {
        services,
        id_map,
        active_mask,
        skipped,
    })
}

fn weekdays(calendar: &RawCalendar) -> Weekday {
    calendar
        .days
        .iter()
        .zip(WEEKDAYS)
        .filter(|&(&runs, _)| runs)
        .fold(Weekday::default(), |days, (_, day)| days.join(day))
}

/// The service ids by index, for error messages.
fn raw_service_ids(id_map: &HashMap<String, ServiceIndex>) -> Vec<&str> {
    let mut ids = vec![""; id_map.len()];
    for (id, index) in id_map {
        ids[index.as_usize()] = id;
    }
    ids
}

/// Builds one bit per service day, each service starting on a byte boundary: set when the service
/// runs that day, from its weekly pattern first and then its calendar date exceptions.
fn build_active_mask(
    services: &mut [Service],
    base_bounds: &[(Date, Date)],
    raw_calendar_dates: &[RawCalendarDate],
    id_map: &HashMap<String, ServiceIndex>,
    ids: &[&str],
) -> Result<BitVec<u8, Msb0>> {
    let mut active_mask = BitVec::<u8, Msb0>::new();

    for (service, &(base_start, base_end)) in services.iter_mut().zip(base_bounds) {
        let days = service.end_date.0 - service.start_date.0 + 1;
        if days > MAX_SERVICE_DAYS {
            return Err(Error::ServiceRangeTooLong {
                service_id: ids[service.index.as_usize()].to_owned(),
                days,
                maximum: MAX_SERVICE_DAYS,
            });
        }
        let days = days as usize;
        let start = active_mask.len();
        let padding = (8 - (start + days) % 8) % 8;
        active_mask.resize(start + days + padding, false);

        for day in service.start_date.0..=service.end_date.0 {
            let date = Date(day);
            let in_base_range = date >= base_start && date <= base_end;
            let runs_today =
                in_base_range && service.weekday.contains(Weekday::from(date.day_of_week()));
            active_mask.set(start + (day - service.start_date.0) as usize, runs_today);
        }

        service.active_mask = ServiceBinarySlice::from_usize(start, days + padding);
    }

    for calendar_date in raw_calendar_dates {
        let Some(index) = id_map.get(&calendar_date.service_id) else {
            continue;
        };
        let service = &services[index.as_usize()];
        let bit_index =
            (service.active_mask.start + calendar_date.date.0 - service.start_date.0) as usize;
        active_mask.set(bit_index, calendar_date.added);
    }

    Ok(active_mask)
}

pub(super) fn build_service_ids(
    services: &mut [Service],
    service_map: &HashMap<String, ServiceIndex>,
) -> (Vec<ServiceIndex>, String) {
    let table = tables::build_id_table::<ServiceIdSlice, _>(service_map, services.len());
    for (service, id) in services.iter_mut().zip(table.slices) {
        service.id = id;
    }
    (table.lookup, table.buffer)
}
