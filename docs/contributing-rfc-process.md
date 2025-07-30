# RFC Process

For changes that will significantly affect Wasmtime's or Cranelift's internals,
downstream projects, contributors, and other stakeholders, a [Bytecode Alliance
RFC](https://github.com/bytecodealliance/rfcs/) should be used to gather
feedback on the proposed change's design and implementation, and to build
consensus.

It is recommended that regular Wasmtime and Cranelift contributors, as well as
any other project stakeholders, subscribe to notifications in [the RFC
repository](https://github.com/bytecodealliance/rfcs/) to stay up to date with
significant change proposals.

## Authoring New RFCs

The RFC repository has two templates that can help you author new proposals:

1. [A draft RFC
   template](https://github.com/bytecodealliance/rfcs/blob/main/template-draft.md),
   for seeking early feedback on ideas that aren't yet fully baked. It is
   expected that as the discussion evolves, these draft RFCs will grow into
   fully-formed RFCs.

2. [A full RFC
   template](https://github.com/bytecodealliance/rfcs/blob/main/template-full.md),
   for building consensus around a mature proposal.

You can also look at historical Wasmtime RFCs in [the repository's `accepted`
subdirectory](https://github.com/bytecodealliance/rfcs/tree/main/accepted) and
their associated discussions in its [merged pull
requests](https://github.com/bytecodealliance/rfcs/pulls?q=is%3Apr+is%3Amerged+)
to gather inspiration for your new RFC. A few good examples include:

* Add a Long Term Support (LTS) Channel of Releases for Wasmtime -
  [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-lts.md) -
  [Discussion](https://github.com/bytecodealliance/rfcs/pull/42)
* Pulley: A Portable, Optimizing Interpreter for Wasmtime -
  [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/pulley.md) -
  [Discussion](https://github.com/bytecodealliance/rfcs/pull/35)
* Debugging Support in Wasmtime -
  [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-debugging.md) -
  [Discussion](https://github.com/bytecodealliance/rfcs/pull/34)
* Redesign Wasmtime's API -
  [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/new-api.md) -
  [Discussion](https://github.com/bytecodealliance/rfcs/pull/11)

After creating a pull request for your new RFC, advertise its existence by
creating a new thread in the relevant
[Zulip](https://bytecodealliance.zulipchat.com/) channels (e.g. `#wasmtime` or
`#cranelift`).
