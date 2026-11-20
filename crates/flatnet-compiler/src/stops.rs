use std::collections::{BTreeMap, HashMap};

use flatnet_core::models::{
    Coordinate, LocationType, Optional, RouteType, SearchSlice, SearchStop, Sentinel, Slice,
    SliceBuilder, Stop, StopIdSlice, StopIndex, StringSlice,
};

use crate::{records::RawStop, tables};

pub(super) fn build_stops(
    raw_stops: &[RawStop],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> (Vec<Stop>, HashMap<String, StopIndex>) {
    let mut id_map: HashMap<String, StopIndex> = HashMap::with_capacity(raw_stops.len());

    let mut stops: Vec<_> = raw_stops
        .iter()
        .enumerate()
        .map(|(position, stop)| {
            let coordinate = match (stop.latitude, stop.longitude) {
                (Some(latitude), Some(longitude)) => {
                    Optional::new(Coordinate::new(latitude, longitude))
                }
                _ => Optional::new(Coordinate::NONE),
            };
            let index = StopIndex::from(position);

            id_map.insert(stop.id.clone(), index);

            let code = stop
                .code
                .as_ref()
                .map(|code| slice_builder.add(code))
                .into();

            let name = stop
                .name
                .as_ref()
                .map(|name| slice_builder.add(name))
                .into();

            let description = stop
                .description
                .as_ref()
                .map(|description| slice_builder.add(description))
                .into();

            Stop::new(
                StopIdSlice::NONE,
                index,
                Optional::new(StopIndex::NONE),
                name,
                description,
                code,
                coordinate,
                stop.location_type,
            )
        })
        .collect();

    for (stop, raw_stop) in stops.iter_mut().zip(raw_stops) {
        let parent = raw_stop
            .parent_station
            .as_ref()
            .and_then(|parent_id| id_map.get(parent_id).copied());
        if let Some(parent_index) = parent {
            stop.parent_index = Optional::new(parent_index);
        }
    }

    (stops, id_map)
}

pub(super) fn build_stop_ids(
    stops: &mut [Stop],
    stop_map: &HashMap<String, StopIndex>,
) -> (Vec<StopIndex>, String) {
    let table = tables::build_id_table::<StopIdSlice, _>(stop_map, stops.len());
    for (stop, id) in stops.iter_mut().zip(table.slices) {
        stop.id = id;
    }
    (table.lookup, table.buffer)
}

/// Groups stops that share a name, so a search for the name finds every platform of a station.
pub(super) fn build_stop_search(stops: &[Stop]) -> (Vec<SearchStop>, Vec<StopIndex>) {
    // Ordered so the same input always gives the same bytes.
    let mut grouped_stops: BTreeMap<StringSlice, Vec<StopIndex>> = BTreeMap::new();

    for stop in stops {
        if let Some(name) = search_name(stop, stops) {
            grouped_stops.entry(name).or_default().push(stop.index);
        }
    }

    let mut search_stops: Vec<SearchStop> = Vec::with_capacity(grouped_stops.len());
    let mut search_to_stops: Vec<StopIndex> = Vec::with_capacity(stops.len());

    for (position, (name, group)) in grouped_stops.into_iter().enumerate() {
        search_stops.push(SearchStop::new(
            position.into(),
            name,
            SearchSlice::from_usize(search_to_stops.len(), group.len()),
            RouteType::NONE,
        ));
        search_to_stops.extend_from_slice(&group);
    }

    (search_stops, search_to_stops)
}

/// The name a stop is searched under: its own for a station, or for a stop that has no parent
/// station, otherwise its parent's. An entrance or boarding area with no parent isn't a place to
/// search for.
fn search_name(stop: &Stop, stops: &[Stop]) -> Option<StringSlice> {
    let is_part_of_something = stop.location_type == LocationType::STATION_ENTRANCE
        || stop.location_type == LocationType::BOARDING_AREA;
    let is_named_place = stop.location_type == LocationType::STOP_AREA
        || (stop.parent_index.is_none() && !is_part_of_something);
    if is_named_place && let Some(name) = stop.name.as_option() {
        return Some(name);
    }
    let parent = stop.parent_index.as_option()?;
    stops[parent.as_usize()].name.as_option()
}
