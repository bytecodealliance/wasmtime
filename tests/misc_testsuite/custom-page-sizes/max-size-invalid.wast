;;! custom_page_sizes = true
;;! hogs_memory = true

;; FIXME(WebAssembly/custom-page-sizes#45): unclear what the semantics of this
;; test should be. For now Wasmtime traps so this tests that traps happen, but
;; this may change in the specification itself. Either way the problem here is
;; that with 1-byte pages an allocation of 0xffff_ffff bytes is
;; indistinguishable from a growth failure which returns -1. Somehow this needs
;; reconciliation and for now it's done as a trap.

(assert_trap
  (module
    (memory 0xffff_ffff (pagesize 1))
  )
  "memory minimum size of 4294967295 pages exceeds memory limits")

(module $m
  (memory (export "memory") 0xffff_fffe (pagesize 1))
)

(module
  (import "m" "memory" (memory 0 (pagesize 1)))

  (func (export "grow") (param i32) (result i32)
    local.get 0
    memory.grow)
)

(assert_trap (invoke "grow" (i32.const 1)) "disallowing growth to 0xffffffff bytes based on page size")
(assert_trap (invoke "grow" (i32.const 2)) "disallowing growth to 0x100000000 bytes based on page size")
(assert_trap (invoke "grow" (i32.const 100)) "disallowing growth to 0x100000062 bytes based on page size")
(assert_trap (invoke "grow" (i32.const -1)) "disallowing growth to 0x1fffffffd bytes based on page size")
(assert_return (invoke "grow" (i32.const 0)) (i32.const -2))
