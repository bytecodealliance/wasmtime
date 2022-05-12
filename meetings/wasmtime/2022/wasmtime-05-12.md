# May 12th Wasmtime project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
    1. _Submit a PR to add your announcement here_
1. Other agenda items
    1. Discuss extending the pollers API
    1. Discuss shared memory implementation (i.e., changes to `VMContext`)
    1. _Submit a PR to add your item here_

## Notes

### Attendees

- acrichton
- cfallin
- fitzgen
- bnjbvr
- saulcabrera
- gkulakowksi
- dgohman
- abrown
- jlbirch
- LGR

### Notes

1. extending the pollers API
   - Dan: Profian has interesting constraints: encryption inside an enclave.
     Polling on encrypted bytes ready is not the same as plaintext ready.
   - Dan: poll() should work for OS-level things and synthesized things as
     well.
   - Dan: timerfd is one possibility
   - Dan: request from Profian is maybe a trait at wasmtime level
   - Dan: maybe WASI virtualization is a better answer
   - Alex: could they override just one function in WASI interface (poll)?
   - George: sounds like that's what they're doing now, too much work, too
     intrusive, hard to maintain
   - Dan: or maybe Cargo feature in Wasmtime with totally custom thing for
     enclaves?
   - Alex: basically what I'm saying, but poll function overridden at Linker,
     no need for special stuff in Wasmtime internals
   - Dan: second half of discussion: poll to be replaced by wait function in
     WASI on canonical ABI / new async stuff
   - Alex: final state is everything is async at embedder API level; not sure
     about transition path
   - Dan: sounds right, TLS thing can be done in Rust async eventually.
     Question is transitional plan
   - Dan: maybe this means we have a bit higher tolerance for now for temporary
     workarounds
   - Andrew: what does the async timeline look like?
   - Alex: working on it, lots of spec work, still a bit unsure
   - Dan: recently started working on a "preview 2" snapshot of WASI, based on
     some new async stuff; timeline is months

2. Shared memory implementation (i.e., changes to `VMContext`)
   - Andrew: [slides](wasmtime-05-12-slides.pdf)
   - Andrew: working on shared memory in Wasmtime, help from Alex.
   - Andrew: main problem is how to share `current_length` in
     `VMMemoryDefinition`, which has a separate copy in each instance
   - Dan: could we "notify", push new lengths to all copies?
   - Chris: we could also lazily update: on trap, look at central
     atomically-updated length and pull in new one if spurious trap. Or:
     generate a slowpath that fetches the atomic and updates, for dynamic
     bounds checks?
   - Alex: would be concerned about code size
   - Chris: make `current_length` a `*const AtomicUsize`, always fetch?
   - Nick: concerned about double-indirection cost
   - Alex: we have double-indirection today for some (imported) memories
   - Nick: benchmarks would be interesting
   - (lots of discussion about tradeoffs, fell behind on notes)
   - Nick: interesting case where mprotect happens before length field
     increments, other threads observe working accesses but read smaller
     memory.size; need lock to protect size too (need memory.size hostcall for
     correctness)
   - Takeaways: Andrew will look at perf overhead, and also figure out why V8
     is not doing paging tricks for shared memories (are we missing something)
   - Ben: we should look at what SpiderMonkey does too
