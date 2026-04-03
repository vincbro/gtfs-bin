use crate::models::{
    Date, Sentinel, Service, ServiceBinarySlice, ServiceIdSlice, ServiceIdx, WeekdaySet,
};
use bitvec::{bitvec, order::Msb0};
use std::collections::HashMap;

pub fn build_services(
    raw_calendar: &[gtfs_structures::Calendar],
    raw_calendar_dates: &[gtfs_structures::CalendarDate],
) -> (Vec<Service>, HashMap<String, ServiceIdx>, Vec<u8>) {
    let mut id_map: HashMap<String, ServiceIdx> = HashMap::new();

    // We build the calendar first, then agregate it with calendar dates
    let mut services: Vec<_> = raw_calendar
        .iter()
        .enumerate()
        .map(|(i, service)| {
            let idx = ServiceIdx(i as u32);
            id_map.insert(service.id.to_string(), idx);
            Service {
                id: ServiceIdSlice::NONE,
                idx,
                start_date: service.start_date.into(),
                end_date: service.end_date.into(),
                active_mask: ServiceBinarySlice::NONE,
                weekdays: WeekdaySet::new()
                    .with_monday(service.monday)
                    .with_tuesday(service.tuesday)
                    .with_wednesday(service.wednesday)
                    .with_thursday(service.thursday)
                    .with_friday(service.friday)
                    .with_saturday(service.saturday)
                    .with_sunday(service.sunday),
                _pad: [0_u8; 3],
            }
        })
        .collect();

    // GTFS standard states that a GTFS set can have either, only calendar or only calendar dates or both
    // aggregating the date here makes building the binary arr later easier
    for calendar_date in raw_calendar_dates.iter() {
        let date = Date::from(calendar_date.date);
        if let Some(idx) = id_map.get(&calendar_date.service_id) {
            let service = &mut services[idx.to_usize()];

            // Safety check to make sure our range is logical
            if date < service.start_date {
                service.start_date = date;
            } else if service.end_date < date {
                service.end_date = date;
            }
        } else {
            let idx = ServiceIdx(services.len() as u32);
            id_map.insert(calendar_date.service_id.to_string(), idx);
            services.push(Service {
                id: ServiceIdSlice::NONE,
                idx,
                start_date: date,
                end_date: date,
                active_mask: ServiceBinarySlice::NONE,
                weekdays: WeekdaySet(u8::MIN),
                _pad: [0_u8; 3],
            });
        }
    }

    // Building the binary map of each service, each bit in the binary map is a day from start_date to end_date, 0 is not running and 1 is running

    let mut active_mask = bitvec![u8, Msb0; 0_u8, 0];
    for service in services.iter_mut() {
        let days = (service.end_date.0 - service.start_date.0) as usize;
        let start = active_mask.len();
        let padding = 8 - ((start + days) % 8);

        active_mask.resize(start + days + padding, true);
        for i in service.start_date.0..service.end_date.0 {
            let idx = (i - service.start_date.0) as usize;
            let date = Date(i);
            let day_of_week = date.get_day_of_week();
            _ = active_mask.replace(idx, service.weekdays.get_day(day_of_week));
        }

        service.active_mask = ServiceBinarySlice {
            start: start as u32,
            count: (days + padding) as u32,
        }
    }

    for calendar_date in raw_calendar_dates.iter() {
        if let Some(idx) = id_map.get(calendar_date.service_id.as_str()) {
            let service = &mut services[idx.to_usize()];
            let date: Date = calendar_date.date.into();
            let active = match calendar_date.exception_type {
                gtfs_structures::Exception::Added => true,
                gtfs_structures::Exception::Deleted => false,
            };
            let idx = (date.0 - service.start_date.0) as usize;
            _ = active_mask.replace(idx, active);
        }
    }

    println!("{} bit mask", active_mask.len());

    (services, id_map, vec![])
}
