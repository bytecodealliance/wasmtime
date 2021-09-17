# September 16 Wasmtime project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
    1. _Sumbit a PR to add your announcement here_
1. Other agenda items
    1. _Sumbit a PR to add your item here_

## Notes

* No agenda, but a few things that would be useful maybe:
   * Wasmtime 1.0 rfc
   * C API function call improvements
   * CI for arm, and in particular M1
* Alex: 1.0 rfc is now in FCP
   * Main change is that we’ve dropped LTS releases since no one is asking for it
   * Releases every 4 weeks
   * CVEs/bugs will backport to the current release
   * If in the future we decide to add LTS, then we will be more informed
   * Backports should be relatively easy since they’re only a month old
* Till: should we backport CVEs to at least two releases?
* Alex: we guarantee one release, but can do more at our discretion
* Till: because the rfc entered FCP, it will likely merge in ten days
   * We don’t have a timeline for when we actually do the 1.0 release yet and all the automation around releases
* Alex: excited to remove lightbeam
* Till: also old backend is something we should discuss
   * It has recently been a maintenance burden, would like not to have that burden, but this requires more discussion
   * In particular Ben previously mentioned better compile times on embark’s code base with the old backend
* Ben: we are actually using the new backend now, but are concerned with compile times
* Till: previously mentioned that you were developing with the old backend but releasing with the new, is that still the case?
* Ben: no, we have switched over to the new backend
* Till: might actually not be any old backend users anymore then
* Alex: update on the C API and function calls
   * Has been fast in rust for a while
   * C was going through the dynamically checked slow path
   * Motivation: 2 ns to enter/leave in rust vs some number in the hundreds for C
   * Exposed the `*mut 128` buffer where args/rets are written/read to C directly
   * C++ will do the same safety encapsulation that Rust has
   * C doesn’t have these abstraction capabilities, so will always be unchecked and unsafe
   * Now got C down to 10 ns, which is a lot better
   * But there is no inlining (without cross language LTO) which accounts for a lot of the remaining delta
   * There are more possibilities for speed ups here, but is getting harder to maintain and diminishing returns
   * Think this is good for now and we can revisit again if necessary
* Till: one thing about the C API is that using it requires cranelift to be available, unlike when using the rust crate, where you can disable the compiler and only execute precompiled modules, not compile new modules
   * This is something we could fix for the C API but could be a bit more involved
   * Something for the future if someone is motivated
* Till: let’s talk about M1 builds
* Anton: my worry is that we might miss something with qemu
   * Yesterday an OOM test merged but is disabled under qemu
   * Fragile, saw this test start failing when running the test natively
   * When I run it solo, it passes, but as part of the whole test suite it fails
   * Point is: we can miss issues when only using qemu and/or disabling tests under qemu
   * This was on aarch64 in general not M1, fwiw
* Alex: is this the test that allocates a bunch of random stuff?
   * We’ve had to disable these tests before because qemu has issues with virtual memory and eagerly committing it [or something like that]
   * Try limiting parallelism to cut down on memory overhead
   * Hard to consider aarch64 truly tier 1 until we are testing it natively on CI
* Till: do we know if there is any progress on having arm-based runners in github actions ci?
   * Rustc did it but is bespoke and has a huge amount of work that went into it to make it safe to run random PRs
* Alex: best thing I can think of for testing aarch64 in ci would be to have daily builds without cfg’ing things on/off but just let it break and give a notification
   * Easier to fix, generally, when it is a recent regression
* Ben: embark has such a thing, can add other people to the email notifications
* Alex: out of curiosity would it be easy to [something I didn’t catch]
* Ben: can ask
* Till: can we have self-hosted runners that don’t run automatically, but only for trusted folks who have commit access to the repo anyways? Maybe with a button that needs to be pushed?
* Alex: would be great, but is a lot of infra to build. Need tests running daily at least, maybe not necessarily every PR
* Till: get them running is step one, policy for fixes/backouts/etc is step two, who is responsible for being on the hook for regressions, etc
* Alex: we will still run qemu in our CI, so hopefully this is just for the few bugs where that is different between qemu and native
* Till: do we need to build this infra ourselves or has anyone made off the shelf solutions we can set up?
* Alex: I think this is mostly something we would have to set up
* Till: unlikely that this is on the critical path for fastly, but if other people build the CI we can accept that as part of the common maintenance burden that contributing anything to wasmtime carries
* Anton: maybe we can start with just nightly builds before CI
* Alex: i was kind of hoping that we could take the existing nightly M1 builds that embark is doing and add the config to integrate with our CI
* Ben: will see if we can give more access to trusted BA members
   * We don’t have linux aarch64 tho
* Alex: we have a linux aarch64 machine available, we just need to connect this with your existing CI set up
* Ben: our build kite set up is protected behind our firewall
* Alex: we could also have another build kite account, but just want to reuse config so we don’t have to figure it all out ourselves
* Anton: M1 is aarch64-based but linux and macos are not the same
   * Memory page sizes differ
   * Shouldn’t lump them together
* Alex: yes, more testing is better if we can get it
* Till: github actions runner went from pre-release to official release for aarch64 in june [or something]
   * Maybe we can get some info from github on timing for native github aarch64 runners
   * And then maybe we won’t need to build this infra ourselves
   * Although they don’t have M1 releases


### Attendees

### Notes
