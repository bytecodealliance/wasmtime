# Roadmap

Our long-term goal is to build an efficient ZK prover for a broad set of WebAssembly programs.

## Background

Building such prover is challenging because of the impedance mismatch between current ZK systems that can efficiently prove only some types of computations (arithmetic identities, table lookups) and WebAssembly that assumes a modern CPU architecture with support for bitwise operations, floating point numbers and random-access memory.
This could lead to a long proving time, depending on how exactly WASM is mapped to zkASM.
Moreover, WebAssembly instruction set is quite broad and has around [400 instructions](https://webassembly.github.io/spec/core/appendix/index-instructions.html) which increases the scope of the work.
Lastly, because we're aiming for a universal WASM prover that can take as an input a list of WASM programs, any optimizations before the execution of WASM programs (similar to optimizations during compilation from WASM to x86) will also have to be proven with ZK circuits.
In other words, in practice, we will be comparing the proving time for WASM program with the time it takes to execute the compiled and optimized for x86 version of the same program on a regular machine.

## Plan of attack

For all the reasons above, it will likely take a long time to reach the goal and it makes sense to tackle this problem incrementally by iterating these two steps:
1. Increasing the scope of supported workloads
2. Improving the performance of the prover on these workloads

Within each iteration we will be:
- Extending and optimizing the mapping of Cranelift IR to ZK processor
- Adjusting ZK processor to support new workloads more efficiently

We will organize the work around three stages:

## Stage 1
- Timeline: September 2023 - November 2023
- Milestone: https://github.com/near/wasmtime/milestone/1

At this stage we focus on proving individual simple WASM programs.

#### Supported workloads
* [WASM MVP Spec tests](https://github.com/WebAssembly/spec/tree/master/test/core) and [instruction set](https://pengowray.github.io/wasm-ops/)
    * We'll skip floating point operations if we can't find a simple software implementation
    * We will support bulk memory operations (`memory.fill` and `memory.copy`), but initially they could be very inefficient

#### Changes to ZK processor
* Switch from 256 to 64 bit registers
* Have 16 general-purpose registers
* Still based on PIL1

#### Performance goals
* At most 1000x slowdown compared to native execution

#### Possible side projects
* Compressing Merkle proofs for stateless validation

## Stage 2
- Timeline: November 2023 - December 2023
- Milestone: https://github.com/near/wasmtime/milestone/2

At this stage we focus on proving one complex program - WASM interpreter.

This will allow us to have a single circuit that can prove any WASM program or even a batch of different WASM programs.

#### Supported workloads
* WASM interpreter - WASMI (subject to change)

#### Changes to ZK processor
* Registers, memory and instructions optimized for WASM
* Likely based on PIL2

#### Performance goals
* At most 200x slowdown compared to native execution

#### Possible side projects
* EVM interpreter - SputnikVM

## Stage 3
- Timeline: January 2024 - March 2024

At this stage we focus on proving one complex library - NEAR protocol.

#### Supported workloads
* NEAR Protocol implementation
    * We will likely need a `no_std` version of it

#### Changes to ZK processor
* Support for NEAR host functions
* Support for NEAR state storage

#### Performance goals
* At most 100x slowdown compared to native execution

#### Possible side projects
* EVM Protocol implementation

## Risks
These risks apply to all stages of the project:
1. Performance of proving might not be good enough
    * We will start evaluating performance early and build projections for the future
    * We will understand ZK side enough to be realistic about future performance improvements
2. We might have too many bugs because correctly implementing WASM semantics is hard
    * We will make sure to have a good test coverage and use WASM Spec test as a part of CI
3. Some workloads might be impossible to support with current proving technology (e.g. large state, long computations)
    * We will identify max size of state and length of computation for Stage 1 and Stage 2 early and communicate it to ZK VM team
4. We might not be productive with Cranelift codebase
    * We will invest in aligning our solution with abstractions used by other Cranelift backends
    * We will build up a relationship with Cranelift dev team to get necessary support
5. ZK ASM processor might turn out to be not well suited for WASM execution
    * We will consult with ZK specialists who are familiar with building ZK VMs to understand what is needed for ZK VM to be a good compilation target
