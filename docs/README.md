# flatnet documentation

flatnet turns a transit feed into one flat file, and a library maps that file and answers questions about it without parsing anything. These pages cover the whole path: compile a feed, read the file, find stops, apply live updates, and what is inside the file.

Every code example here is checked against the crates, so you can copy it.

## Start here

| I want to... | Read |
|---|---|
| Try it in five minutes | [Getting started](getting-started.md) |
| Understand the model before I write code | [Concepts](concepts.md) |
| Compile a feed from the command line | [The command line](cli.md) |
| Compile from Rust, or from a source that isn't GTFS | [Compiling](compiling.md) |
| Read a file: routes, trips, stop times, shapes | [Reading a file](reading.md) |
| Know what day and time a trip runs | [Dates, times and service days](time-and-service-days.md) |
| Find stops near a point, by name, or on foot | [Finding things](finding-things.md) |
| Apply delays, cancellations and alerts | [Realtime](realtime.md) |
| Copy a worked example | [Recipes](recipes.md) |
| Fix an error message | [Troubleshooting](troubleshooting.md) |
| Know what a corrupt file can and cannot do | [Safety and validation](safety.md) |
| See how the bytes are laid out | [The file format](file-format.md) |
| Know why it is built this way | [Design](design.md) |

## The pages in order

If you read them front to back:

1. [Getting started](getting-started.md): install, compile a feed, open it, ask it something.
2. [Concepts](concepts.md): compile and consume, indices, optional values, parts.
3. [The command line](cli.md): every flag of `flatnet`.
4. [Compiling](compiling.md): `Compiler`, the GTFS reader, your own `Network`, parts, settings and metadata.
5. [Reading a file](reading.md): `Consumer`, the tables and how to walk between them.
6. [Dates, times and service days](time-and-service-days.md): `Date`, `Time`, services and trips past midnight.
7. [Finding things](finding-things.md): the stop grid, name search, places and footpaths.
8. [Realtime](realtime.md): `RealtimeBuilder`, `Realtime`, alerts and sharing between threads.
9. [Recipes](recipes.md): a departure board, a live board, walking neighbours and more.
10. [Safety and validation](safety.md): what `Consumer::new` checks and what that buys you.
11. [The file format](file-format.md): the header, every section and the conventions.
12. [Design](design.md): the decisions behind the format and what is left out.
13. [Troubleshooting](troubleshooting.md): errors, what they mean and what to do.

## Crates

| Crate | What it is |
|---|---|
| `flatnet-cli` | The `flatnet` command, which compiles a feed. |
| `flatnet-compiler` | `Compiler`, which turns a `Network` into the file. With the `gtfs` feature it also has `gtfs::Reader`. |
| `flatnet-core` | The types of the file, `Consumer`, which maps and reads it, and the realtime side. |
| `flatnet-viewer` | An egui app to walk a compiled file on a map. Not published. |

## Where the rest lives

- [README.md](../README.md): the short version, with the installation and the roadmap.
- [TODO.md](../TODO.md): what is done, open and decided against.
- [CONTRIBUTING.md](../CONTRIBUTING.md): setup, the `just` recipes, lints and commits.
- API docs: `cargo doc --open`, or `just docs`.
