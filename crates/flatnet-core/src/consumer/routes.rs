use crate::{
    consumer::Consumer,
    models::{Route, RouteIdSlice, RouteIndex, Slice},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `RouteIndex::NONE`.
    #[must_use]
    pub fn route(&self, index: RouteIndex) -> &'a Route {
        &self.routes[index.as_usize()]
    }

    #[must_use]
    pub fn route_by_id(&self, id: &str) -> Option<&'a Route> {
        self.routes_id_lookup
            .binary_search_by(|&index| {
                let route = self.route(index);
                self.route_id(route.id).cmp(id)
            })
            .ok()
            .map(|position| self.route(self.routes_id_lookup[position]))
    }

    /// # Panics
    ///
    /// Panics if the slice is out of range.
    #[must_use]
    pub fn route_id(&self, id: RouteIdSlice) -> &'a str {
        super::text(&self.route_ids[id.range()])
    }
}
