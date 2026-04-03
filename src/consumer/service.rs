use crate::{
    consumer::Consumer,
    models::{Date, Service, ServiceIdSlice, ServiceIdx, Slice},
};

impl<'a> Consumer<'a> {
    #[inline]
    pub fn service(&self, idx: ServiceIdx) -> &'a Service {
        &self.services[idx.to_usize()]
    }

    pub fn service_by_id(&self, id: &str) -> Option<&'a Service> {
        self.services_id_lookup
            .binary_search_by(|&idx| {
                let service = self.service(idx);
                self.service_id(service.id).cmp(id)
            })
            .ok()
            .map(|idx| self.service(self.services_id_lookup[idx]))
    }

    #[inline]
    pub fn service_id(&self, id: ServiceIdSlice) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(&self.service_ids[id.range()]) }
    }

    pub fn is_service_active(&self, idx: ServiceIdx, date: Date) -> bool {
        let service = self.service(idx);

        if date < service.start_date || date > service.end_date {
            return false;
        }

        let day_offset = (date.0 - service.start_date.0) as u32;
        let bit_idx = service.active_mask.start + day_offset;

        let byte_idx = (bit_idx / 8) as usize;
        let bit_offset = 7 - (bit_idx % 8);

        (self.active_mask[byte_idx] & (1 << bit_offset)) != 0
    }
}
