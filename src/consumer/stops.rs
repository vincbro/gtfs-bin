use crate::{
    consumer::Consumer,
    models::{SearchIdx, SearchStop, Slice, Stop, StopIdSlice, StopIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    pub fn stop(&self, idx: StopIdx) -> &'a Stop {
        &self.stops[idx.as_usize()]
    }

    pub fn stop_by_id(&self, id: &str) -> Option<&'a Stop> {
        self.stops_id_lookup
            .binary_search_by(|&idx| {
                let stop = self.stop(idx);
                self.stop_id(stop.id).cmp(id)
            })
            .ok()
            .map(|idx| self.stop(self.stops_id_lookup[idx]))
    }

    #[inline]
    pub fn stop_id(&self, id: StopIdSlice) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.stop_ids[id.range()]) }
    }

    pub fn search(&self, idx: SearchIdx) -> &SearchStop {
        &self.search_stops[idx.as_usize()]
    }
    pub fn iter_stops_by_search(&self, idx: SearchIdx) -> impl Iterator<Item = &'a Stop> {
        let search = self.search(idx);
        self.search_to_stops[search.stops.range()]
            .iter()
            .copied()
            .map(|stop| self.stop(stop))
    }
}
