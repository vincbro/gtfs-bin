use std::collections::HashMap;

use crate::models::{Sentinel, Service, ServiceIdSlice, ServiceIdx, WeekdaySet};

pub fn build_services(
    raw_services: &[gtfs_structures::Calendar],
) -> (Vec<Service>, HashMap<String, ServiceIdx>) {
    let mut id_map: HashMap<String, ServiceIdx> = HashMap::new();

    let services: Vec<_> = raw_services
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

    (services, id_map)
}
