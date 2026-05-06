use crate::{
    consumer::Consumer,
    models::{Route, RouteIdSlice, RouteIdx, Slice},
};

impl<'a> Consumer<'a> {
    #[inline]
    #[must_use]
    pub fn route(&self, idx: RouteIdx) -> &'a Route {
        &self.routes[idx.as_usize()]
    }

    #[must_use]
    pub fn route_by_id(&self, id: &str) -> Option<&'a Route> {
        self.routes_id_lookup
            .binary_search_by(|&idx| {
                let route = self.route(idx);
                self.route_id(route.id).cmp(id)
            })
            .ok()
            .map(|idx| self.route(self.routes_id_lookup[idx]))
    }

    #[inline]
    #[must_use]
    pub fn route_id(&self, id: RouteIdSlice) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.route_ids[id.range()]) }
    }
}
