# haste

world's fastest dota 2 and deadlock (the game) replay parser. more then two
times faster than comically fast.

haste attempts to squeeze maximum single-core performance from the cpu, which
enables efficient utilization of all cores for parsing multiple replays
simultaneously.

haste does not want to be very user-friendly, it provides you with a relatively
low-level access to correct usable data. it up to you how you structure your
programs. you may choose to build your own nice api layer. anything is possible,
theoretically even something as silly (i love silly) as ecs is doable (think of
bevy game engine).

> [!WARNING]
> there are many `unsafe`s in the codebase, some are rational and explained,
> some need to be let go; public api isn't great and can be considered very
> unstable (e.g. it is not final and parts of it may change dramatically).

## how to

### examples

notable examples to check out for detailed usage:

- [deadlock-position](examples/deadlock-position) demonstrates how to work with
entities and how to get player (or any other entity, if desired) positions in
deadlock (the game);
- [deadlock-gametime](examples/deadlock-gametime) also demonstrates how to work
with entities and how to compute game time (not a very straightforward thing to
do, thanks valve).
- [dota2-allchat](examples/dota2-allchat) shows how to work with packet
messages.

to run these examples navigate to haste directory and run

```console
$ cargo run --package <example-name> -- <path-to-dem-file>
```

### usage

to use haste in your project, you'll need either:
 - `protoc` (protocol buffer compiler) in your `$PATH`, or `$PROTOC` environment
 variable needs point to it
 - or `cmake` (if you don't have `protoc`, it will be compiled for you) and
 `protobuf-src` feature flag enabled

haste is not published to [crates.io](https://crates.io/) (yet?). you can add it
to your `Cargo.toml` as a [git
dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories),

```toml
[dependencies]
haste = { git = "https://github.com/blukai/haste.git" }
```

haste's entity representatin is not debugger / print friendly, you would want to
use [haste-inspector](https://github.com/blukai/haste-inspector) (replay dev
tools) to explore all the entities that are present in replays.

## feature flags

- `deadlock`: enables deadlock protos and some utilities.
- `dota2`: enabled dota2 protos and some utilities.
- `protobuf-src`: enables
[protobuf_src](https://docs.rs/protobuf-src/latest/protobuf_src/) crate which
builds `protoc`.

## benchmarks

TODO: benchmarks and comparisons with other projects such as clarity and manta.

to tease a bit.. as of 25-09-2024 in standard release build with no extra
optimizations nor non-stadard memory allocators:

- 31 minutes deadlock match can be parsed in ~660 ms with ~17.5 mb peak memory
consumtion;
- 38 minutes dota 2 captains mode match - in ~650 ms with ~18 mb peak memory
consumtion.

run time is an average from 10 runs with no warmups. peak memory consumtion is
`time`'s `maximum resident set size` stat.

## motivation, huh?

why create another replay parser?
to prove myself that i'm right (long story; the proof was found quicly, but then
i fell down the rabbit hole of further performance optimizations).

## credits / references

valve's official repos and wiki provide quite a handful of useful information,
but special credits go to [invokr](https://github.com/invokr) who worked on
[dotabuff/manta](https://github.com/dotabuff/manta) and to
[spheenik](https://github.com/spheenik) the creator of
[skadistats/clarity](https://github.com/skadistats/clarity) (i have not personally
interacted with either of them).

other notable resources:

- [ButterflyStats/butterfly](https://github.com/ButterflyStats/butterfly)
- [ValveSoftware/csgo-demoinfo](https://github.com/ValveSoftware/csgo-demoinfo)
- [ValveSoftware/source-sdk-2013](https://github.com/ValveSoftware/source-sdk-2013)
- [SwagSoftware/Kisak-Strike](https://github.com/SwagSoftware/Kisak-Strike)
- [markus-wa/demoinfocs-golang](https://github.com/markus-wa/demoinfocs-golang)
(some info about how pointer serializer fields work)
- [demofile-net](https://github.com/saul/demofile-net) (tv broadcasts)

## performance / profiling

- [hotspot](https://github.com/KDAB/hotspot)
- [heaptrack](https://github.com/KDE/heaptrack)
- [perf](https://perf.wiki.kernel.org/index.php/Main_Page)
- [hyperfine](https://github.com/sharkdp/hyperfine)
- [asheplyakov/branchmiss](https://github.com/asheplyakov/branchmiss)

and some more can probably be found across comments in the codebase.
