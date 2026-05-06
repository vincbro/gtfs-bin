use crate::{
    consumer::Consumer,
    models::{Slice, StopIdx, Transfer, TransferIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    #[must_use]
    pub fn transfer(&self, idx: TransferIdx) -> &'a Transfer {
        &self.transfers[idx.as_usize()]
    }

    /// Transfers from A-B by A idx
    #[inline]
    #[must_use]
    pub fn outbound_transfers_by_stop(&self, idx: StopIdx) -> &'a [Transfer] {
        let slice = self.stop_to_transfer_out[idx.as_usize()];
        &self.transfers[slice.range()]
    }

    #[inline]
    #[must_use]
    pub fn inbound_transfers_by_stop(&self, idx: StopIdx) -> &[TransferIdx] {
        let slice = self.stop_to_transfer_in[idx.as_usize()];
        &self.transfers_in_indencies[slice.range()]
    }

    /// Transfers from A-B by B idx
    pub fn iter_inbound_transfers_by_stop(
        &self,
        idx: StopIdx,
    ) -> impl Iterator<Item = &'a Transfer> {
        self.inbound_transfers_by_stop(idx)
            .iter()
            .copied()
            .map(|idx| self.transfer(idx))
    }
}
