//! The header of the file and the sections it lists, to check the layout against `FORMAT.md`.

use std::time::{SystemTime, UNIX_EPOCH};

use flatnet_core::{
    consumer::Consumer,
    models::{Date, Header, Section},
};

/// What the header says about the file.
#[derive(Debug)]
pub(crate) struct FileInfo {
    pub(crate) path: String,
    pub(crate) len: u64,
    pub(crate) header: Header,
}

/// One section with how much of the file it takes up.
#[derive(Debug)]
pub(crate) struct SectionRow {
    pub(crate) name: &'static str,
    pub(crate) offset: u64,
    pub(crate) count: u64,
    /// The bytes up to the next section, so it includes the zeros that align the next one.
    pub(crate) bytes: u64,
}

// A function can't turn a field name into text, and the viewer lists every section by the name the
// header gives it. `the_list_names_every_section_of_the_header` fails when the two drift apart.
macro_rules! named_sections {
    ($header:expr, $($field:ident),+ $(,)?) => {
        [$((stringify!($field), $header.$field)),+]
    };
}

impl FileInfo {
    /// Reads the header, or returns `None` if the file is too short to have one.
    pub(crate) fn read(file: &[u8], path: String) -> Option<Self> {
        let bytes = file.get(..size_of::<Header>())?;
        Some(Self {
            path,
            len: file.len() as u64,
            header: bytemuck::pod_read_unaligned(bytes),
        })
    }

    /// Returns the sections in the order they lie in the file.
    pub(crate) fn sections(&self) -> Vec<SectionRow> {
        let header = &self.header;
        let named: [(&'static str, Section); 39] = named_sections!(
            header,
            stops,
            stop_ids,
            stops_id_lookup,
            routes,
            route_ids,
            route_id_lookup,
            trips,
            trip_ids,
            trip_id_lookup,
            services,
            service_ids,
            service_id_lookup,
            active_mask,
            stop_to_trips,
            stop_to_trips_lookup,
            search_stops,
            search_to_stops,
            route_to_trips,
            route_to_trips_lookup,
            stop_times,
            shapes,
            shape_points,
            trip_shapes,
            trip_patterns,
            trip_patterns_stop_seq,
            trip_patterns_trip_seq,
            trip_to_trip_pattern,
            stop_to_trip_pattern,
            stop_to_trip_pattern_lookup,
            transfers,
            stop_to_transfers_out,
            transfers_in_indices,
            stop_to_transfers_in,
            strings,
            timezone,
            stop_grid,
            grid_stops,
            metadata,
            metadata_json,
        );
        let mut sorted = named.to_vec();
        sorted.sort_by_key(|(_, section)| section.offset);
        let ends: Vec<u64> = sorted
            .iter()
            .skip(1)
            .map(|(_, section)| section.offset)
            .chain([self.len])
            .collect();
        sorted
            .iter()
            .zip(ends)
            .map(|(&(name, section), end)| SectionRow {
                name,
                offset: section.offset,
                count: section.count,
                bytes: end - section.offset,
            })
            .collect()
    }
}

/// Today as a day, in UTC. The agency's own day can differ by a few hours, which doesn't matter
/// for showing how long a file has left.
pub(crate) fn today() -> Date {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    Date(u32::try_from(seconds / 86_400).unwrap_or(0))
}

/// Names the first and last day any service runs, and whether the file has gone stale.
pub(crate) fn activity(consumer: &Consumer<'_>, today: Date) -> String {
    let Some((first, last)) = consumer.service_range() else {
        return "no services, so stale".to_owned();
    };
    let state = if consumer.is_stale(today) {
        format!("STALE, ended {} days ago", today.0 - last.0)
    } else if today < first {
        format!("starts in {} days", first.0 - today.0)
    } else {
        format!("{} days left", last.0 - today.0)
    };
    format!("{first} to {last} ({state})")
}

/// Names when the file was compiled, or says it was not recorded.
pub(crate) fn compiled_at(seconds: u64) -> String {
    if seconds == 0 {
        return "not recorded".to_owned();
    }
    let day = Date(u32::try_from(seconds / 86_400).unwrap_or(0));
    let in_day = seconds % 86_400;
    format!("{day} {:02}:{:02} UTC", in_day / 3_600, in_day % 3_600 / 60)
}

#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;

    use super::*;

    #[test]
    fn the_list_names_every_section_of_the_header() {
        let info = FileInfo {
            path: String::new(),
            len: 0,
            header: Header::zeroed(),
        };

        // The magic, the version, the feed dates, the file length and the checksum are not sections.
        let fixed = 4 + 4 + 4 + 4 + 8 + 8;
        assert_eq!(
            size_of::<Header>(),
            fixed + info.sections().len() * size_of::<Section>(),
            "a section was added to the header and not to the list"
        );
    }
}
