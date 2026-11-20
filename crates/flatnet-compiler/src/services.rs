use bitvec::{order::Msb0, vec::BitVec};
use flatnet_core::models::{
    BitMask, Date, Sentinel, Service, ServiceBinarySlice, ServiceIdSlice, ServiceIndex, Slice,
    Weekday,
};

use crate::{Error, Result, network::ServiceRecord, tables};

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

/// The services and the bits of days each service runs.
pub(super) struct BuiltServices {
    pub services: Vec<Service>,
    pub active_mask: BitVec<u8, Msb0>,
}

/// Builds the services, one bit for each of a service's days, each service starting on a byte
/// boundary: set when the service runs that day, from its weekly pattern first and then its
/// exceptions. A service runs from its first day to its last, counting its exceptions.
pub(super) fn build_services(records: &[ServiceRecord]) -> Result<BuiltServices> {
    // The dates the weekly pattern applies to, before exceptions stretch a service's range. A
    // service with no week never runs on a weekly pattern, so its base range is empty.
    let mut base_bounds: Vec<(Date, Date)> = Vec::with_capacity(records.len());
    let mut services: Vec<Service> = Vec::with_capacity(records.len());

    for (position, record) in records.iter().enumerate() {
        let (mut start, mut end, weekday, bounds) = match record.weekly {
            Some(weekly) => (
                weekly.start,
                weekly.end,
                weekdays(weekly.days),
                (weekly.start, weekly.end),
            ),
            None => (
                Date(u32::MAX),
                Date(u32::MIN),
                Weekday::default(),
                (Date(u32::MAX), Date(u32::MIN)),
            ),
        };
        for &(date, _) in &record.exceptions {
            start = start.min(date);
            end = end.max(date);
        }
        if end < start {
            return Err(Error::EmptyService {
                service_id: record.id.clone(),
            });
        }
        base_bounds.push(bounds);
        services.push(Service::new(
            ServiceIdSlice::NONE,
            ServiceIndex::from(position),
            ServiceBinarySlice::NONE,
            start,
            end,
            weekday,
        ));
    }

    let active_mask = build_active_mask(&mut services, &base_bounds, records)?;
    Ok(BuiltServices {
        services,
        active_mask,
    })
}

fn weekdays(days: [bool; 7]) -> Weekday {
    days.iter()
        .zip(WEEKDAYS)
        .filter(|&(&runs, _)| runs)
        .fold(Weekday::default(), |days, (_, day)| days.join(day))
}

fn build_active_mask(
    services: &mut [Service],
    base_bounds: &[(Date, Date)],
    records: &[ServiceRecord],
) -> Result<BitVec<u8, Msb0>> {
    let mut active_mask = BitVec::<u8, Msb0>::new();

    for ((service, &(base_start, base_end)), record) in
        services.iter_mut().zip(base_bounds).zip(records)
    {
        let days = service.end_date.0 - service.start_date.0 + 1;
        if days > MAX_SERVICE_DAYS {
            return Err(Error::ServiceRangeTooLong {
                service_id: record.id.clone(),
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

    for (service, record) in services.iter().zip(records) {
        for &(date, added) in &record.exceptions {
            let bit_index = (service.active_mask.start + date.0 - service.start_date.0) as usize;
            active_mask.set(bit_index, added);
        }
    }

    Ok(active_mask)
}

pub(super) fn build_service_ids(
    services: &mut [Service],
    records: &[ServiceRecord],
) -> (Vec<ServiceIndex>, String) {
    let ids: Vec<&str> = records.iter().map(|service| service.id.as_str()).collect();
    let table = tables::build_id_table::<ServiceIdSlice, _>(&ids);
    for (service, id) in services.iter_mut().zip(table.slices) {
        service.id = id;
    }
    (table.lookup, table.buffer)
}
