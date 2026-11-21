# Safety and validation

A `.flnt` file is read in place, with no parser between the bytes and your code. That is only acceptable if a bad file can't hurt you. This page says what `Consumer::new` promises, how that is tested, and where the promise ends.

## The promise

> After `Consumer::new` returns `Ok`, for any bytes it was given, no accessor reads outside the file, and none panics on an index the file itself gave you.

A corrupt, truncated or hostile file is an `Err` from `Consumer::new`. It is never undefined behavior and never a panic later.

What the check covers:

| Check | Means |
|---|---|
| Header | The magic is `FLNT`, the version is the one this build reads, and the declared length is the real length. |
| Sections | Every section lies inside the file, is aligned, and holds a whole number of items. Arithmetic on offsets and counts can't overflow. |
| Text | Every id and string section is valid UTF-8, and every slice into one starts and ends on a character. |
| Positions | Every item's own `index` is its position. |
| References | Every index and slice stored in a table points inside the table it names. |
| Lookups | Every lookup has one entry per item of the table it indexes. The id lookups are sorted, so binary search finds ids. |
| Orders | Routes list their own trips from the earliest start to the latest. A place's footpaths run nearest first. The stop grid is sorted. |
| Timings | Each pattern's trips lie back to back in `timings`, each with a `Timing` for every stop of the pattern, and no trip is earlier than the one before it at any stop. |
| Services | Each service's mask covers all of its days, and its dates run forward. |
| Shapes | Each shape has points and a finite length. A trip with shapes has one fewer than it has stop times. |
| Metadata | There is exactly one item, its text lies inside `strings`, its settings are ones a compile would accept, and the active range has both ends or neither. |
| Parts | A part the header says is absent is empty, no unknown bit is set, and search comes with text. |

The full list is in [the file format](file-format.md#what-consumernew-guarantees), and the code is `crates/flatnet-core/src/consumer/validate.rs`.

## What it does not promise

- **That the file is the one the compiler wrote.** A file can be internally consistent and still not be the one you meant. `Consumer::verify_checksum` compares a hash of every byte after the header, and finds any changed byte. It reads the whole file, so it isn't part of `new`. Use it when a file arrives from somewhere: a download, a cache, a shared disk.
- **That the data is right.** The compiler can only be as good as the feed. A stop in the wrong place stays in the wrong place.
- **That an index you invented is valid.** `consumer.stop(StopIndex::from(10_000_000_usize))` panics. So does `StopIndex::NONE`. The `# Panics` section of each accessor says so.
- **That the file does not change under you.** Mapping is `unsafe` in `memmap2` for this reason. Replace files by writing a new file and renaming it, never by writing into a file that is mapped.

## `new_unvalidated`

`Consumer::new_unvalidated` checks the header, the length and that each section lies inside the file, and skips the rest. Use it for a file your own program wrote a moment ago, such as in a build step or a test.

It never causes undefined behavior: every slice is in bounds and aligned. What a damaged file can do is make an accessor panic or return nonsense. If the file came from anywhere else, use `new`.

## The unsafe code

There is very little.

- `Optional<T>` implements `bytemuck::Pod` and `Zeroable` by hand, because bytemuck's traits can only be implemented unsafely. It is `repr(transparent)` over a `Pod` type, so it has the same layout and no padding.
- Mapping a file is `unsafe`, and the callers do it: your code, the `inspect` example, the viewer.
- Everything else is safe Rust. The tables are cast from bytes with `bytemuck::try_cast_slice`, which checks size and alignment.

## How it is tested

The validators are only as good as the tests that try to get past them. Each of these is a property test with shrinking, and the cases that failed once are committed (`crates/flatnet-core/tests/*.proptest-regressions`) so they run every time.

| Test | What it does |
|---|---|
| `fuzz.rs` | Opens arbitrary bytes, files with bytes changed, truncated files, files with table entries set to small, huge and just-past-the-end values, and files with words swapped. Then it calls every accessor on whatever opened. |
| `fuzz_parts.rs` | The same damage on a file compiled with every combination of parts, and on files from arbitrary networks. A changed body must fail its checksum. |
| `fuzz_network.rs` | Compiles arbitrary networks, with NaN positions, parent loops, impossible dates and odd times. The compiler returns an error or a file `Consumer::new` accepts. |
| `fuzz_compiler.rs`, `fuzz_patterns.rs`, `fuzz_shapes.rs`, `fuzz_metadata.rs` | Damaged and generated GTFS feeds. Checks times read back as written, patterns keep the order, cut shapes join into one path, and metadata comes back merged. |
| `fuzz_services.rs` | Which days a service runs, against a plain calendar. |
| `fuzz_footpaths.rs` | Places and footpaths, against a scan of every pair of stops, including across the 180th meridian and by a pole. |
| `fuzz_queries.rs` | Stops near and in a box anywhere on the globe, ids, trips by start, trips at a stop and name search, against a plain scan. |
| `fuzz_rt.rs`, `fuzz_rt_neutral.rs` | Arbitrary realtime feeds: any date, timestamp, time zone and position. The state must not change size, and nothing panics. |
| `fuzz_dates.rs` | Every day up to year 9999 round-trips, text parsing, and arbitrary bytes decoded as a realtime feed. |

Each validator also has a test that damages one thing and expects that one check to catch it, so removing a check makes a test fail. `just check` runs these at a few hundred cases each. `just check-full` runs them at 40,000, and is run before a release and after touching the validators, the compiler or the realtime builder.

## Compile-side limits

A hostile feed can't make the compiler write a bad file, because the compiler returns an error for what it can't represent. It can still be expensive:

- A service that runs for more than 36,600 days (about a hundred years) is refused, because every day costs a bit.
- Thousands of trips on one stop sequence that all overtake each other make the pattern search quadratic. That is a compile-time cost only, and it is listed in [TODO.md](../TODO.md).

Compile feeds you trust, or compile them in a process you can time out.

Next: [the file format](file-format.md), or [Troubleshooting](troubleshooting.md) if you are looking at an error.
