# zkASM Cranelift Backend

This is an experimental backend from Cranelift IR to [Polygon zkASM](https://wiki.polygon.technology/docs/zkevm/zkProver/the-processor/) virtual machine aimed to enable Zero-Knowledge proofs about execution of general programs.

## zkASM

zkASM is a register machine with memory that is programmed in a [textual assembly language](https://wiki.polygon.technology/docs/zkevm/zkASM/basic-syntax/).

It has some notable differences from traditional ISAs:
- ZK processor is [fully defined in software](https://github.com/0xPolygonHermez/zkevm-proverjs/blob/main/pil/main.pil) in a language called PIL which is well suited for generating ZK proofs. As a consequence, we have a lot of freedom to change it as we see fit (as long as it is still cheap to prove)
- The relative costs of many instructions are very different from traditional ISAs, e.g. bitwise operations (AND, OR, XOR, NOT) are 4-16 times more expensive than arithmetic operations (ADD, SUB, NEG)

Some relevant tooling to work with zkASM:
- Simulators in [JavaScript](https://github.com/0xPolygonHermez/zkevm-proverjs) and [C++](https://github.com/0xPolygonHermez/zkevm-prover)
- [Text Assembly Parser](https://github.com/0xPolygonHermez/zkasmcom)

For more details, see the [machine spec](https://github.com/0xPolygonHermez/zkevm-techdocs/blob/main/zkevm-architecture/v.1.1/zkevm-architecture.pdf).
