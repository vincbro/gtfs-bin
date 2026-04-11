use crate::{
    consumer::Consumer,
    models::{Slice, StopIdx, Transfer, TransferIdx},
};

impl<'a> Consumer<'a> {
    pub fn transfer(&self, idx: TransferIdx) -> &'a Transfer {
        &self.transfers[idx.as_usize()]
    }

    /// Transfers from A-B by A idx
    pub fn outbound_transfers_by_stop(&self, idx: StopIdx) -> &'a [Transfer] {
        let slice = self.stop_to_transfer_out[idx.as_usize()];
        &self.transfers[slice.range()]
    }

    /// Transfers from A-B by B idx
    pub fn iter_inbound_transfers_by_stop(
        &self,
        idx: StopIdx,
    ) -> impl Iterator<Item = &'a Transfer> {
        let slice = self.stop_to_transfer_in[idx.as_usize()];
        self.transfers_in_indencies[slice.range()]
            .iter()
            .copied()
            .map(|idx| self.transfer(idx))
    }
}
