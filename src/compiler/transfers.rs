use std::collections::HashMap;

use crate::models::{RouteIdx, StopIdx, Transfer, TripIdx};

// pub(crate) fn build_transfers(
//     raw_transfers: &[gtfs_structures::RawTransfer],
//     stop_map: &HashMap<String, StopIdx>,
//     trip_map: &HashMap<String, TripIdx>,
//     route_map: &HashMap<String, RouteIdx>,
// ) -> Result<(Vec<Transfer>), gtfs_structures::Error> {
//     let transfers: Vec<_> = raw_transfers
//         .iter().filter_map(||)
//         .map(|transfer| Transfer {
//             from_stop_idx: ,
//             to_stop_idx: todo!(),
//             from_route_idx: todo!(),
//             to_route_idx: todo!(),
//             from_trip_idx: todo!(),
//             to_trip_idx: todo!(),
//             min_transfer_time: todo!(),
//             transfer_type: todo!(),
//             _pad: todo!(),
//         })
//         .collect();

//     Ok((trips, id_map))
// }
