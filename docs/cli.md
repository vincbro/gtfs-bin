# The command line

`flatnet` compiles a GTFS feed into a `.flnt` file. It is one command with a handful of flags.

```sh
flatnet -i path/to/gtfs.zip -o network.flnt
```

It prints how long reading and compiling took and how long writing took, and nothing else on success. On failure it prints the error with the file it was about.

## Flags

| Flag | Default | Meaning |
|---|---|---|
| `-i`, `--input <PATH>` | required | A GTFS feed: a `.zip`, or a folder of `.txt` files. |
| `-o`, `--output <PATH>` | `output.flnt` | Where to write the file. |
| `--meta <KEY=VALUE>` | none | Adds a text entry to the file's JSON metadata. Repeat it for more. |
| `--meta-json <FILE>` | none | A file with a JSON object to put in the metadata. `--meta` replaces its keys. |
| `--footpath-meters <METERS>` | `400` | The longest straight line of a footpath between two places. At most 50,000. |
| `--footpath-max <COUNT>` | `32` | The most footpaths one place keeps, the nearest first. |
| `--without <PART>` | none | Leaves a part out. Repeat it for more. |
| `-V`, `--version` | | Prints the version. |
| `-h`, `--help` | | Prints the flags. |

## Examples

A feed in a zip, with defaults:

```sh
flatnet -i sl.zip -o sl.flnt
```

A feed in a folder:

```sh
flatnet -i ./gtfs-folder -o city.flnt
```

Put your own information in the file. Anything you add here comes back from `Consumer::metadata_json`:

```sh
flatnet -i sl.zip -o sl.flnt --meta region=stockholm --meta build=1234
```

```rust
use flatnet_core::consumer::Consumer;

fn read_meta(consumer: &Consumer<'_>) {
    // `{"build":"1234","region":"stockholm"}`, with the keys sorted.
    println!("{:?}", consumer.metadata_json());
}
```

A longer walk to the next place, but fewer of them:

```sh
flatnet -i sl.zip -o sl.flnt --footpath-meters 800 --footpath-max 10
```

## Leaving parts out

`--without` takes one of `shapes`, `search`, `text`, `footpaths` and `ids`. A smaller file opens faster and uses less memory. Sizes on SL's regional feed, 44.3 MB with everything (at the time of writing, see [TODO.md](../TODO.md)):

| Flags | Size |
|---|---|
| none | 44.3 MB |
| `--without shapes` | 38.9 MB |
| `--without ids` | 41.2 MB |
| `--without text` | 43.9 MB |
| `--without shapes --without text --without ids` | 35.4 MB, opens in 6 ms |

Things to know:

- **`text` takes `search` with it**, because a place is found by its name and there is nothing to search without names.
- **Without `ids`** the `*_by_id` lookups find nothing, so a realtime feed matches no trip. Keep `ids` if you apply realtime data.
- **Without `text`** a headsign is no longer part of what makes a trip pattern, so two trips that differ only in headsign can share a pattern.

What each part holds: [Concepts](concepts.md#parts).

## Just recipes

If you work in the repository, `just` wraps the common commands. Run `just` alone to list them.

| Recipe | What it does |
|---|---|
| `just compile FEED [OUT]` | Compiles a feed with the release build. |
| `just inspect FILE [near LAT LON METERS \| search TEXT]` | Opens a file, times it, and can run a query. |
| `just apply FILE TripUpdates.pb [more.pb ...]` | Applies realtime files to a compiled file and reports what matched. |
| `just view [FILE]` | Opens the viewer: a window to walk the tables and see stops and shapes on a map. |
| `just dummy` | Compiles the small feed in `data/dummy` into `data/dummy.flnt`, to try the viewer on. |
| `just dist` | Builds the shipped binary with fat LTO into `target/dist/flatnet`. |

## Inspecting a file

`inspect` is an example program that opens a file the way your code would and prints what it finds.

```sh
just inspect network.flnt
just inspect network.flnt near 59.3303 18.0586 250
just inspect network.flnt search centralen
```

It reports the table sizes, the metadata, how long opening and validating took, and the checksum.

## Exit status

`0` on success. Any error, such as a missing input, a malformed value or an unwritable output, prints a message and exits non-zero. The messages are explained in [Troubleshooting](troubleshooting.md#compile-errors).
