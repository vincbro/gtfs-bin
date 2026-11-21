//! Property tests that compile arbitrary networks: what the compiler accepts, `Consumer::new`
//! must accept too, and every accessor must work on it.

mod common;

use common::arbitrary;
use flatnet_compiler::Compiler;
use flatnet_core::{consumer::Consumer, models::Parts};
use proptest::prelude::*;

proptest! {

    #[test]
    fn what_the_compiler_accepts_the_consumer_accepts(
        network in arbitrary::network(),
        parts in 0_u64..32,
    ) {
        let Ok(bytes) = Compiler::new().parts(Parts(parts)).compile(network) else {
            return Ok(());
        };
        let (_file, mmap) = common::map(&bytes).expect("the file maps");

        let consumer = Consumer::new(&mmap);

        prop_assert!(consumer.is_ok(), "the compiler wrote a file it can't read: {:?}", consumer.err());
        common::exercise(&consumer.expect("checked above"));
    }
}
