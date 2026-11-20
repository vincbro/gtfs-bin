use flatnet_core::models::{
    Duration, Optional, StopIndex, Transfer, TransferIndex, TransferSlice, TripIndex,
};
use rayon::slice::ParallelSliceMut;

use crate::{network::TransferRecord, tables};

/// The transfers and the two lookups from a stop to its transfers.
pub(super) struct BuiltTransfers {
    /// Sorted by the stop they start at.
    pub transfers: Vec<Transfer>,
    /// By stop: the run of `transfers` that starts there.
    pub outbound: Vec<TransferSlice>,
    /// Indices into `transfers`, sorted by the stop they end at.
    pub inbound_indices: Vec<TransferIndex>,
    /// By stop: the run of `inbound_indices` that ends there.
    pub inbound: Vec<TransferSlice>,
}

pub(super) fn build_transfers(records: &[TransferRecord], stop_count: usize) -> BuiltTransfers {
    let trip = |position: Option<usize>| {
        position
            .map(TripIndex::from)
            .map_or_else(Optional::default, Optional::new)
    };
    let mut transfers: Vec<_> = records
        .iter()
        .map(|transfer| {
            Transfer::new(
                StopIndex::from(transfer.from_stop),
                StopIndex::from(transfer.to_stop),
                trip(transfer.from_trip),
                trip(transfer.to_trip),
                transfer.min_transfer_time.map(Duration).into(),
                transfer.transfer_type,
            )
        })
        .collect();

    transfers.par_sort_unstable_by_key(|transfer| transfer.from_stop_index);
    let outbound = tables::slices_by_key(
        transfers.iter().map(|transfer| transfer.from_stop_index),
        stop_count,
    );

    let mut inbound_indices: Vec<_> = (0..transfers.len()).map(TransferIndex::from).collect();
    inbound_indices.par_sort_unstable_by_key(|index| transfers[index.as_usize()].to_stop_index);
    let inbound = tables::slices_by_key(
        inbound_indices
            .iter()
            .map(|index| transfers[index.as_usize()].to_stop_index),
        stop_count,
    );

    BuiltTransfers {
        transfers,
        outbound,
        inbound_indices,
        inbound,
    }
}
