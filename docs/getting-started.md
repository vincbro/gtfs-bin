# Getting started

In this page you compile a feed, open the result from Rust, and ask it three questions. It takes about five minutes.

## 1. Install

You need [Rust](https://rustup.rs) 1.95 or later.

```sh
cargo install flatnet-cli     # the `flatnet` command
```

In your own project:

```sh
cargo add flatnet-core        # read a file
cargo add memmap2             # map it
cargo add flatnet-compiler    # only if you compile from Rust
```

The `gtfs-rt` feature of `flatnet-core` (on by default) needs `protoc` to build: `brew install protobuf` or `apt install protobuf-compiler`.

## 2. Compile a feed

Any GTFS feed works, a `.zip` or a folder of `.txt` files.

```sh
flatnet -i path/to/gtfs.zip -o network.flnt
```

You get one file, `network.flnt`. The same feed always gives the same bytes. More options are in [the command line](cli.md).

## 3. Open it

```rust
use std::fs::File;

use flatnet_core::consumer::Consumer;
use memmap2::Mmap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("network.flnt")?;
    // SAFETY: the file is only read, and nothing modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file)? };
    let consumer = Consumer::new(&mmap)?;

    println!(
        "{} stops, {} routes, {} trips",
        consumer.stops().len(),
        consumer.routes().len(),
        consumer.trips().len(),
    );
    Ok(())
}
```

`Consumer::new` checks the whole file before it returns, so a damaged file is an error here and never a panic later. On a regional network of 2.5 million stop times that takes about 10 ms. The `Consumer` borrows the map, so keep `mmap` alive as long as you use it.

## 4. Ask it things

Look something up by the id the feed used, and read its name:

```rust
use flatnet_core::consumer::Consumer;

fn describe(consumer: &Consumer<'_>) {
    let Some(stop) = consumer.stop_by_id("9021001002010000") else {
        println!("no such stop");
        return;
    };
    let name = consumer
        .stop_info(stop.index)
        .name
        .as_option()
        .map(|slice| consumer.string(slice));
    println!("stop {:?} is called {name:?}", stop.index);
}
```

Walk from a route to its trips:

```rust
use flatnet_core::consumer::Consumer;

fn trips_of(consumer: &Consumer<'_>, route_id: &str) {
    let Some(route) = consumer.route_by_id(route_id) else {
        return;
    };
    for trip in consumer.iter_trips_by_route(route.index) {
        println!("trip {}", consumer.trip_id(trip.id));
    }
}
```

Find the stops within 250 meters of a point, nearest first:

```rust
use flatnet_core::consumer::Consumer;

fn near(consumer: &Consumer<'_>) {
    // The buffer is yours: keep it between calls and nothing allocates.
    let mut found = Vec::new();
    consumer.stops_near(59.3303, 18.0586, 250.0, &mut found);
    for (stop, meters) in &found {
        println!("{:?} is {meters:.0} m away", stop);
    }
}
```

## Where to next

- The model behind these calls: [Concepts](concepts.md).
- Everything a `Consumer` can do: [Reading a file](reading.md) and [Finding things](finding-things.md).
- Live delays on top of the file: [Realtime](realtime.md).
- A worked departure board: [Recipes](recipes.md).
