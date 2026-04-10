use crate::models::{
    Date, Sentinel, Service, ServiceBinarySlice, ServiceIdSlice, ServiceIdx, Slice, SliceBuilder,
    WeekdaySet,
};
use bitvec::{bitvec, order::Msb0, vec::BitVec};
use rayon::slice::ParallelSliceMut;
use std::collections::HashMap;

pub fn build_services(
    raw_calendar: &[gtfs_structures::Calendar],
    raw_calendar_dates: &[gtfs_structures::CalendarDate],
) -> (Vec<Service>, HashMap<String, ServiceIdx>, BitVec<u8, Msb0>) {
    let mut id_map: HashMap<String, ServiceIdx> = HashMap::new();

    let mut base_bounds: Vec<(Date, Date)> = Vec::with_capacity(raw_calendar.len());

    // We build the calendar first, then agregate it with calendar dates
    let mut services: Vec<_> = raw_calendar
        .iter()
        .enumerate()
        .map(|(i, service)| {
            let idx = ServiceIdx(i as u32);
            id_map.insert(service.id.to_string(), idx);
            let start_date = service.start_date.into();
            let end_date = service.end_date.into();
            base_bounds.push((start_date, end_date));
            Service {
                id: ServiceIdSlice::NONE,
                idx,
                start_date,
                end_date,
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
            let service = &mut services[idx.as_usize()];

            // Safety check to make sure our range is logical
            if date < service.start_date {
                service.start_date = date;
            } else if service.end_date < date {
                service.end_date = date;
            }
        } else {
            let idx = ServiceIdx(services.len() as u32);
            id_map.insert(calendar_date.service_id.to_string(), idx);
            base_bounds.push((Date(u32::MAX), Date(u32::MIN)));
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
    for (i, service) in services.iter_mut().enumerate() {
        let days = (service.end_date.0 - service.start_date.0) as usize + 1;
        let start = active_mask.len();
        let padding = 8 - ((start + days) % 8);

        active_mask.resize(start + days + padding, false);

        let (base_start, base_end) = base_bounds[i];

        for i in service.start_date.0..=service.end_date.0 {
            let idx = start + (i - service.start_date.0) as usize;
            let date = Date(i);
            let runs_today = if date >= base_start && date <= base_end {
                service.weekdays.get_day(date.get_day_of_week())
            } else {
                false
            };

            _ = active_mask.replace(idx, runs_today);
        }

        service.active_mask = ServiceBinarySlice {
            start: start as u32,
            count: (days + padding) as u32,
        }
    }

    for calendar_date in raw_calendar_dates.iter() {
        if let Some(idx) = id_map.get(calendar_date.service_id.as_str()) {
            let service = &mut services[idx.as_usize()];
            let date: Date = calendar_date.date.into();
            let active = match calendar_date.exception_type {
                gtfs_structures::Exception::Added => true,
                gtfs_structures::Exception::Deleted => false,
            };
            let idx = (service.active_mask.start + date.0 - service.start_date.0) as usize;
            _ = active_mask.replace(idx, active);
        }
    }

    (services, id_map, active_mask)
}

pub(crate) fn build_service_ids(
    services: &mut [Service],
    service_map: &HashMap<String, ServiceIdx>,
) -> (Vec<ServiceIdx>, String) {
    let mut id_builder = SliceBuilder::with_capacity(36 * services.len());
    for (id, idx) in service_map.iter() {
        services[idx.as_usize()].id = id_builder.add(id.as_str());
    }

    let service_ids = id_builder.take();

    // Build binary search friendly id lookup
    let mut service_id_lookup: Vec<_> = (0..services.len()).map(|i| ServiceIdx(i as u32)).collect();
    service_id_lookup.par_sort_unstable_by(|a, b| {
        let id_a = &service_ids[services[a.as_usize()].id.range()];
        let id_b = &service_ids[services[b.as_usize()].id.range()];
        id_a.cmp(id_b)
    });

    (service_id_lookup, service_ids)
}
