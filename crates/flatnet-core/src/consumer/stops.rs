use crate::{
    consumer::Consumer,
    models::{SearchIndex, SearchStop, Slice, Stop, StopIdSlice, StopIndex},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn stop(&self, index: StopIndex) -> &'a Stop {
        &self.stops[index.as_usize()]
    }

    #[must_use]
    pub fn stop_by_id(&self, id: &str) -> Option<&'a Stop> {
        self.stops_id_lookup
            .binary_search_by(|&index| {
                let stop = self.stop(index);
                self.stop_id(stop.id).cmp(id)
            })
            .ok()
            .map(|position| self.stop(self.stops_id_lookup[position]))
    }

    /// # Panics
    ///
    /// Panics if the slice is out of range.
    #[must_use]
    pub fn stop_id(&self, id: StopIdSlice) -> &'a str {
        super::text(&self.stop_ids[id.range()])
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn search(&self, index: SearchIndex) -> &'a SearchStop {
        &self.search_stops[index.as_usize()]
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn stops_by_search(&self, index: SearchIndex) -> &'a [StopIndex] {
        let search = self.search(index);
        &self.search_to_stops[search.stops.range()]
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_stops_by_search(&self, index: SearchIndex) -> impl Iterator<Item = &'a Stop> {
        self.stops_by_search(index)
            .iter()
            .map(|&stop_index| self.stop(stop_index))
    }
}
