# February 3rd Wasmtime project call

**See the [instructions](../README.md) for details on how to attend**

### Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
    1. _Submit a PR to add your announcement here_
1. Other agenda items
    1. cfallin: update on memfd, CoW, lazy init
    1. _Submit a PR to add your item here_

### Attendees
- abrown
- acrichton
- fitzgen
- cfallin
- lgr
- jlbirch
- harold
- sunfish

### Notes

cfallin: several instantiation performance PRs in the pipeline (or merged):
 - memfd: anonymous memory file for the heap contents is `mmap`-ed in; can quickly clear any changes in the overlay to reuse it
 - pooling allocator slot allocation: use the same slot for the module if possible
 - other changes to instantiation performance: making more things lazy (e.g., `funcref` tables)

cfallin: with the `funcref` tables change, Wasmtime can instantiate the Spidermonkey Wasn module ~3x faster (72us to 22us)

acrichton: could be even faster

jlbirch: in wnat scenarios will we see these benefits?

cfallin: with the memfd changes there are benefits:
 - when instantiating just once there is no need to do eager init of memory contents (just `mmap`)
 - caveat: first instantiation might be net same as current implementation...
 - additional wins if you can reuse the module's slot
 - table init chnage is a benefit universally

jlbirch: when benchmarking, if we observer inconsistent instantiation times, how can we tell if the fast or slow path is taken?

cfallin: we could expose stats in the affinity allocator

acrichton: note that the module must be the right shape for affinity to work well--dynamic init messes things up (?); this could be improved to recognize more module shapes

abrown: what are some best-case numbers for instantiation?

cfallin: instantiation should be nothing more than an `madvise` call and a few pointer stores

acrichton: for a multi-MB Wasm file, not even pooling, instantiation can be 10us; caveat: single-threadeded result, may find more issues with concurrency

lgr: so this pushes towards doing everything on the fly?

cfallin: yes, majority of work should happen in a new instance--security benefits...

abrown: ittapi crate (for VTune support) has been improved by me, bnjbvr and jlbirch; almost ready for inclusion in the default Wasmtime build--should we support more OS than Linux, macOS, Windows?

acrichton: those are the currently supported OS; that should be fine
