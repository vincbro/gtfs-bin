use std::collections::HashMap;

use crate::models::{Sentinel, Service, ServiceIdSlice, ServiceIdx};

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
                start_day: 0,
                end_day: 0,
                weekdays: 0_u8,
                _pad: [0_u8; 3],
            }
        })
        .collect();

    (services, id_map)
}
