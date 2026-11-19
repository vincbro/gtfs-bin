use crate::compiler::tables;
use crate::models::{
    BitMask, Date, Sentinel, Service, ServiceBinarySlice, ServiceIdSlice, ServiceIndex, Slice,
    Weekday,
};
use bitvec::{order::Msb0, vec::BitVec};
use std::collections::HashMap;

pub fn build_services(
    raw_calendar: &[gtfs_structures::Calendar],
    raw_calendar_dates: &[gtfs_structures::CalendarDate],
) -> (
    Vec<Service>,
    HashMap<String, ServiceIndex>,
    BitVec<u8, Msb0>,
) {
    let mut id_map: HashMap<String, ServiceIndex> = HashMap::with_capacity(raw_calendar.len());
    // The dates the weekly pattern applies to, before calendar dates stretch a service's range.
    let mut base_bounds: Vec<(Date, Date)> = Vec::with_capacity(raw_calendar.len());
    let mut services: Vec<Service> = Vec::with_capacity(raw_calendar.len());

    // A GTFS set can have only a calendar, only calendar dates, or both. The calendar goes first,
    // and the calendar dates are folded into it afterwards.
    for (position, calendar) in raw_calendar.iter().enumerate() {
        let index = ServiceIndex::from(position);
        id_map.insert(calendar.id.clone(), index);
        let start_date = Date::from(calendar.start_date);
        let end_date = Date::from(calendar.end_date);
        base_bounds.push((start_date, end_date));
        services.push(Service::new(
            ServiceIdSlice::NONE,
            index,
            ServiceBinarySlice::NONE,
            start_date,
            end_date,
            weekdays(calendar),
        ));
    }

    for calendar_date in raw_calendar_dates {
        let date = Date::from(calendar_date.date);
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

    let active_mask = build_active_mask(&mut services, &base_bounds, raw_calendar_dates, &id_map);
    (services, id_map, active_mask)
}

fn weekdays(calendar: &gtfs_structures::Calendar) -> Weekday {
    [
        (calendar.monday, Weekday::MONDAY),
        (calendar.tuesday, Weekday::TUESDAY),
        (calendar.wednesday, Weekday::WEDNESDAY),
        (calendar.thursday, Weekday::THURSDAY),
        (calendar.friday, Weekday::FRIDAY),
        (calendar.saturday, Weekday::SATURDAY),
        (calendar.sunday, Weekday::SUNDAY),
    ]
    .into_iter()
    .filter(|&(runs, _)| runs)
    .fold(Weekday::default(), |days, (_, day)| days.join(day))
}

/// Builds one bit per service day, each service starting on a byte boundary: set when the service
/// runs that day, from its weekly pattern first and then its calendar date exceptions.
fn build_active_mask(
    services: &mut [Service],
    base_bounds: &[(Date, Date)],
    raw_calendar_dates: &[gtfs_structures::CalendarDate],
    id_map: &HashMap<String, ServiceIndex>,
) -> BitVec<u8, Msb0> {
    let mut active_mask = BitVec::<u8, Msb0>::new();

    for (service, &(base_start, base_end)) in services.iter_mut().zip(base_bounds) {
        let days = (service.end_date.0 - service.start_date.0) as usize + 1;
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
        let date = Date::from(calendar_date.date);
        let active = matches!(
            calendar_date.exception_type,
            gtfs_structures::Exception::Added
        );
        let bit_index = (service.active_mask.start + date.0 - service.start_date.0) as usize;
        active_mask.set(bit_index, active);
    }

    active_mask
}

pub fn build_service_ids(
    services: &mut [Service],
    service_map: &HashMap<String, ServiceIndex>,
) -> (Vec<ServiceIndex>, String) {
    let table = tables::build_id_table::<ServiceIdSlice, _>(service_map, services.len());
    for (service, id) in services.iter_mut().zip(table.slices) {
        service.id = id;
    }
    (table.lookup, table.buffer)
}
