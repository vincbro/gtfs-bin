use crate::{
    consumer::Consumer,
    models::{Date, Service, ServiceIdSlice, ServiceIndex, Slice},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `ServiceIndex::NONE`.
    #[must_use]
    pub fn service(&self, index: ServiceIndex) -> &'a Service {
        &self.services[index.as_usize()]
    }

    #[must_use]
    pub fn service_by_id(&self, id: &str) -> Option<&'a Service> {
        self.services_id_lookup
            .binary_search_by(|&index| {
                let service = self.service(index);
                self.service_id(service.id).cmp(id)
            })
            .ok()
            .map(|position| self.service(self.services_id_lookup[position]))
    }

    /// # Panics
    ///
    /// Panics if the slice is out of range.
    #[must_use]
    pub fn service_id(&self, id: ServiceIdSlice) -> &'a str {
        super::text(&self.service_ids[id.range()])
    }

    /// Returns whether the service runs on `date`.
    ///
    /// A trip whose service the file doesn't define has `ServiceIndex::NONE`, and that never runs.
    #[must_use]
    pub fn is_service_active(&self, index: ServiceIndex, date: Date) -> bool {
        let Some(service) = self.services.get(index.as_usize()) else {
            return false;
        };

        if date < service.start_date || date > service.end_date {
            return false;
        }

        let day_offset = date.0 - service.start_date.0;
        let bit_index = service.active_mask.start + day_offset;

        let byte_index = (bit_index / 8) as usize;
        let bit_offset = 7 - (bit_index % 8);

        (self.active_mask[byte_index] & (1 << bit_offset)) != 0
    }
}
