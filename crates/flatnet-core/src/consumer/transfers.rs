use crate::{
    consumer::Consumer,
    models::{Slice, StopIndex, Transfer, TransferIndex},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn transfer(&self, index: TransferIndex) -> &'a Transfer {
        &self.transfers[index.as_usize()]
    }

    /// Returns the transfers that start at the stop.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn outbound_transfers_by_stop(&self, index: StopIndex) -> &'a [Transfer] {
        let slice = self.stop_to_transfer_out[index.as_usize()];
        &self.transfers[slice.range()]
    }

    /// Returns the indices of the transfers that end at the stop.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn inbound_transfers_by_stop(&self, index: StopIndex) -> &'a [TransferIndex] {
        let slice = self.stop_to_transfer_in[index.as_usize()];
        &self.transfers_in_indices[slice.range()]
    }

    /// Returns the transfers that end at the stop.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `StopIndex::NONE`.
    pub fn iter_inbound_transfers_by_stop(
        &self,
        index: StopIndex,
    ) -> impl Iterator<Item = &'a Transfer> {
        self.inbound_transfers_by_stop(index)
            .iter()
            .map(|&transfer_index| self.transfer(transfer_index))
    }
}
