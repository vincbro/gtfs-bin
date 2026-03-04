use crate::{
    consumer::Consumer,
    models::{Slice, Stop, StopIdSlice, StopIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    pub fn stop(&self, idx: StopIdx) -> &'a Stop {
        &self.stops[idx.to_usize()]
    }

    pub fn stop_by_id(&self, id: &str) -> Option<&'a Stop> {
        self.stops_id_lookup
            .binary_search_by(|&idx| {
                let stop = self.stop(idx);
                self.stop_id(stop.id).cmp(id)
            })
            .ok()
            .map(|idx| &self.stops[idx])
    }

    #[inline]
    pub fn stop_id(&self, id: StopIdSlice) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.stop_ids[id.range()]) }
    }
}
