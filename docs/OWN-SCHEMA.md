# Owning the schema: GTFS, GTFS-RT, NeTEx and SIRI compared

A design note, not a decision. It asks whether flatnet should stop being "a compiled GTFS" and become "a compiled transit network" with GTFS, GTFS-RT, NeTEx and SIRI as importers.

Facts about GTFS-bin come from `TODO.md` and `docs/FORMAT.md`. Facts about NeTEx and SIRI are from the CEN specs as I know them (TS 16614 and TS 15531), at the level of concepts, not field by field. Check a concept against the spec or the Nordic profile before building on it.

## Decision (2026-11-20)

The goal is a tool that makes working with a transit network easy and fast, however large the network is. It is **not** a goal to cover everything GTFS, GTFS-RT, NeTEx or SIRI can say.

- The format is ours. A field is in it because a consumer needs it, and out of it when none does, whatever the source calls it.
- There is no promise of a one to one mapping with GTFS. A reader may drop what we don't use, derive what we do, and the format may add values no source has.
- Each field has to earn its place: what reads it, and what it costs in size and time. Speed is a goal, so a change that can be measured is measured on the SL feed and the generated one (`just bench`).
- The comparison below is a menu to pick from, not a checklist to complete.

## 1. The four formats in one line each

| Format | What it is | Shape | Scope |
|---|---|---|---|
| GTFS | Static timetable for journey planners | Flat CSV tables | Stops, routes, trips, times, calendars. Small and easy |
| GTFS-RT | Live overlay on a GTFS feed | Protobuf, keyed by GTFS ids | Trip updates, vehicle positions, alerts |
| NeTEx (CEN TS 16614) | The full description of a public transport network | Nested XML in "frames" | Network, timetable, fares, stations, operations, vehicles |
| SIRI (CEN TS 15531) | Live data, the counterpart of NeTEx | XML (and a "lite" REST/JSON form) | Estimated timetable, vehicle monitoring, situations, stop monitoring, connections, facilities |

GTFS is to NeTEx what GTFS-RT is to SIRI: the small thing that won, next to the large thing that is complete.

## 2. What each one has and lacks

### GTFS (static)

Has: agencies, stops with a parent station and entrances, routes, trips, stop times, weekly calendars with exceptions, shapes, transfers, frequencies, pathways and levels, fares (v1 and v2), translations, booking rules and flex areas, feed validity.

Lacks:
- A **line** above the route, or a **journey pattern** (the stop sequence); both are implied by trips.
- A **logical stop** apart from a **physical one**. A GTFS stop is both.
- **Operator** apart from **authority**. There is only `agency`.
- **Vehicles**, rolling stock, vehicle schedules beyond `block_id`, drivers.
- **Validity per object** and **versions**. Only the whole feed has dates.
- **Namespaces** for ids. Two feeds can both have a stop `1`.
- Rich **accessibility** (a flag, not a description of the step or the lift).
- Stable ids across feed versions. Nothing promises them.

### GTFS-RT

Has: delays and times per stop, cancelled, skipped and added trips, vehicle position, speed, bearing, occupancy, alerts with an informed entity and an active period, a vehicle descriptor, trip modifications (experimental).

Lacks:
- **Aimed, expected and actual** time as separate values per call. It sends one number.
- **Arrival status** per call (arrived, departed, no report).
- **Platform or quay change** as a first class thing (only `assigned_stop_id` in stop time properties).
- A reliable **trip key**. It matches by `trip_id`, or by route, start time and date, and sometimes it is ambiguous.
- **Connection** protection, **facility** status, and rich disruption causes.

### NeTEx

Has, beyond GTFS:
- `Line`, `Route`, `JourneyPattern`, `ServiceJourney`, `TimetabledPassingTime`.
- `ScheduledStopPoint` (logical) mapped to `Quay` and `StopPlace` (physical) by `PassengerStopAssignment`.
- `DayType`, `OperatingDay`, `OperatingPeriod`.
- `Authority`, `Operator`, `Organisation`, contact details.
- `Site`, entrances, `PathLink` and navigation paths for station interiors.
- Vehicle types, vehicle scheduling (blocks, duties), service links.
- A full fare model: zones, products, usage parameters, sales.
- Versions, validity conditions, codespaces on every id.

Lacks:
- Anything live. That is SIRI.
- Simplicity. The same fact can be written in several ways, so everyone uses a **profile** (the EU one, EPIP, or the Nordic one).
- A compact or fast form. It is XML.

### SIRI

Has, beyond GTFS-RT:
- Estimated Timetable (ET): per call, aimed, expected and actual times, arrival status, platform, boarding.
- Vehicle Monitoring (VM), Stop Monitoring (SM), Situation Exchange (SX, rich disruptions with scopes and affects), Production Timetable (PT), Connection Timetable and Monitoring (CT, CM), General Message (GM), Facility Monitoring (FM).
- `DatedVehicleJourneyRef`: a key for one run of a trip on one day.
- Extra journeys, cancellations and replacements as defined cases.
- Subscription and request/response.

Lacks:
- The network. It refers to NeTEx (or GTFS) ids and carries no stops or lines of its own.
- Geometry.
- A small footprint.

## 3. The Venn diagram

```
          ┌──────────────────────────────────────────────────────────────┐
          │                           NeTEx + SIRI                       │
          │  line · journey pattern · logical stop vs quay · operator    │
          │  vs authority · versions & validity · codespaces · vehicles  │
          │  duties · station navigation · full fares · aimed/expected/  │
          │  actual · arrival status · connections · facilities ·        │
          │  DatedVehicleJourneyRef · rich situations                    │
          │                                                              │
          │      ┌──────────────────────────────────────────────┐        │
          │      │              SHARED CORE                     │        │
          │      │  stop (+ station hierarchy) · route/line ·   │        │
          │      │  trip/journey · stop time/passing time ·     │        │
          │      │  service calendar + exceptions · transfer/   │        │
          │      │  connection · agency/operator · shape/       │        │
          │      │  geometry · accessibility (basic) · alerts/  │        │
          │      │  situations · vehicle position · delay ·     │        │
          │      │  cancelled / added trip · occupancy          │        │
          │      │                                              │        │
          │      │   ┌──────────────────────────────────────┐   │        │
          │      │   │      GTFS + GTFS-RT only             │   │        │
          │      │   │  frequencies as a first class form · │   │        │
          │      │   │  one flat CSV per concept · feed_info│   │        │
          │      │   │  · shape_dist_traveled · trip        │   │        │
          │      │   │  modifications (experimental)        │   │        │
          │      │   └──────────────────────────────────────┘   │        │
          │      └──────────────────────────────────────────────┘        │
          └──────────────────────────────────────────────────────────────┘
```

GTFS has almost nothing NeTEx lacks. The "GTFS only" ring is mostly a matter of form: NeTEx can say all of it, in a different way. So the shared core is almost all of GTFS, and the real work in a second importer is the NeTEx and SIRI region around it.

## 4. Where flatnet stands

`[x]` is in the format today, `[~]` is partly there, `[ ]` is not.

### Shared core

| Concept | State | Notes |
|---|---|---|
| Stops, parent stations | [x] | `stops`, `search_stops`, grid index |
| Routes | [x] | |
| Trips, stop times, pickup/drop-off | [x] | Times interpolated between timed stops |
| Calendars and exceptions | [x] | Expanded to `active_mask`, one bit per day |
| Transfers, in-seat transfers | [x] | |
| Agency, time zone, feed validity | [x] | In `metadata` |
| Shapes | [x] | Cut at stops, stored once per hop |
| Journey pattern | [~] | `trip_patterns` is derived by grouping trips. Not a source concept |
| Alerts, vehicle position, delays, cancelled, added | [x] | In `Realtime`, outside the file, one per service day |
| Occupancy | [~] | On vehicle positions. Not per stop time |
| Accessibility | [ ] | Not in the format |

### GTFS only, or the NeTEx region

| Concept | State |
|---|---|
| Frequencies, timepoint, booking rules, pathways, levels, fares | [ ] open in `TODO.md` |
| Translations, attributions | [ ] |
| Line above route | [ ] |
| Logical stop vs quay | [ ] |
| Operator vs authority | [ ] (one agency) |
| Versions, validity per object, codespaces | [ ] |
| Aimed, expected and actual time | [ ] (one `Realtime` delay) |
| Arrival status per call | [ ] |
| `DatedVehicleJourneyRef` as the trip key | [~] The matching rules cover `trip_id` and the route/start/date form |
| Vehicles, duties | [ ] |
| Connections as a live thing | [ ] |
| Facilities | [ ] |

The honest summary: flatnet covers the **shared core** and the routing half of the **GTFS only** set. It covers none of the NeTEx region.

## 5. Should flatnet own its schema?

My view: **yes to the idea, no to the rewrite, and not yet to the rename.**

### For

- The format is already not GTFS. It has patterns, a grid, cut shapes, a search index and a validated layout. Calling it a compiled GTFS undersells it.
- Every importer written against the binary format directly would repeat the work of sorting, interning and indexing. A shared builder in front of the writer avoids that.
- NeTEx shows three distinctions that would fix real GTFS limits: logical stop vs physical stop, line vs route vs pattern, and a trip key per day. Of those, the trip key is already the pain in `Realtime` (the matching code and the one unmatched update in SL's feed).
- A router like `blaise` wants a network, not a source format.

### Against

- A schema that fits GTFS, NeTEx, SIRI and "anything else we come across" is a design for an unknown set. The usual failures are the lowest common denominator, where it loses what NeTEx had, and the union, where it becomes NeTEx.
- A transpiler is only as good as the target. Each format added makes the schema harder to change, and each change touches every importer.
- `TODO.md` has open work that real feeds already need: `timepoint`, `frequencies`, booking rules. SL has no use for NeTEx today. Build for SL's feed first (this is also what the memory note on the flat layout says: measure before moving).
- NeTEx is large. A full importer is a project of its own. A profile (Nordic, since SL and Trafiklab publish it) is the only sane target.
- A rename costs links, crates.io history and the docs, and gives nothing until there is a second importer.

### What I would do instead: make the seam, not the swap

The layout can stay. What changes is **where the compiler's input ends and the format begins**.

```
 GTFS reader ─┐
 NeTEx reader ├─►  Network  (typed, in memory, id-namespaced)  ─►  Writer  ─►  .bin
 other reader ┘            ▲
                           │ validated once, here
```

1. `Network` is the in-memory transit schema: the types in `records.rs` made source-neutral. It is the thing to design carefully. It is not the binary format and it is not `Pod`.
2. The writer takes a `Network` and produces the binary file, exactly as it does now. The flat layout, `Optional<T>`, the validators and the fuzz tests stay.
3. A reader per source produces a `Network`. GTFS is first. Porting it is a pure refactor, so the SL output must be byte for byte the same, which the deterministic output rule makes testable. That check ends at the first deliberate format change; after it, the tests are about what the format promises, not about matching GTFS.
4. Realtime gets the same split: a source-neutral `Update` (aimed, expected, actual, status per call) and a decoder per source (GTFS-RT, SIRI-ET, SIRI-VM, SIRI-SX). `Realtime` applies updates and does not know the wire format.
5. The binary format gains only what a second source needs, one field at a time, with a version bump and a validator each.

Then the NeTEx importer is a new reader, not a new format, and the schema earns its keep with two sources before it is called final.

### Structural changes worth making in the schema

These are cheap in `Network` and expensive later in the file.

| Change | Why |
|---|---|
| Ids carry a **namespace** (`source:id`) | Two feeds merge, and NeTEx ids are already `codespace:Type:id` |
| **Stop place** (physical) and **stop point** (the thing a trip calls at) as two things, linked | NeTEx has it. GTFS can fill both from one row. Platform changes then become an assignment, not a new stop |
| **Line** → **route** → **pattern** → **trip** | `trip_patterns` becomes a real table that trips point at, not a derived group |
| A **run key**: trip + service day, as one id | The SIRI `DatedVehicleJourneyRef`. Ends the ambiguous matching |
| **Operator** and **authority** as separate | One agency is a special case of both |
| **Validity** on an object, not only the feed | NeTEx has it, and it lets a merged network carry two versions |
| Time as **aimed / expected / actual** in `Realtime` | One delay number cannot carry SIRI-ET |
| Accessibility as a small set of values, not a flag | Both families have more than a boolean |
| Frequencies stored as a headway with a pattern | The open `TODO.md` question. NeTEx has `HeadwayJourneyGroup`. Decide it once, for both |

### What I would leave out of the schema

Fares, vehicle scheduling, duties, driver data, facility monitoring and station navigation graphs. Each is a project, none is needed to route, and a source-neutral model for them is the part most likely to be wrong. Add one when a consumer asks for it.

## 6. A name

If the schema is owned, `flatnet` stops being accurate, but this is the last thing to do. Do the rename when the second reader works, so the new name describes something that exists.

Directions, none checked for availability on crates.io:

- **Descriptive:** `transit-bin`, `tbin`, `netbin`. Clear, forgettable.
- **Short and plain:** `halt`, `stopbin`, `timetable`. Stops and timetables are what it is made of.
- **In the family of `blaise` and `wend`:** a single word chosen like those two, not describing the contents.

My pick would be to keep `flatnet` as the CLI and crate for the GTFS reader (`flatnet-compiler` already exists as a crate), and name the format and core after what they are. The workspace is already split into core, compiler and cli, so the format name and the importer name can differ without a rewrite.

## 7. Suggested order

1. Finish what SL needs: `timepoint`, `frequencies` decision. (`TODO.md`)
2. Introduce `Network` between the GTFS reader and the writer. Same output, byte for byte.
3. Add the cheap schema changes above: namespaces, run key, stop place and stop point, line.
4. Split realtime into `Update` and a GTFS-RT decoder.
5. Write a Nordic profile NeTEx reader as the second source, and SIRI-ET as the second realtime source.
6. Only then decide the name, and bump the format.
