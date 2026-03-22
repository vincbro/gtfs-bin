use crate::models::{Date, Sentinel, Service, ServiceIdSlice, ServiceIdx, WeekdaySet};
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
                weekdays: WeekdaySet(u8::MAX),
                _pad: [0_u8; 3],
            });
        }
    }
    (services, id_map, vec![])
}
