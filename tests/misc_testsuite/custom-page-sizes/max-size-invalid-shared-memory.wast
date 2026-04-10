;;! custom_page_sizes = true
;;! hogs_memory = true
;;! threads = true

(assert_trap
  (module
    (memory 0xffff_ffff 0xffff_ffff shared (pagesize 1))
  )
  "memory minimum size of 4294967295 pages exceeds memory limits")

(module $m
  (memory (export "memory") 0xffff_fffe 0xffff_ffff shared (pagesize 1))
)

(module
  (import "m" "memory" (memory 0 0xffff_ffff shared (pagesize 1)))

  (func (export "grow") (param i32) (result i32)
    local.get 0
    memory.grow)
)

(assert_return (invoke "grow" (i32.const 1)) (i32.const -1))
(assert_return (invoke "grow" (i32.const 2)) (i32.const -1))
(assert_return (invoke "grow" (i32.const 100)) (i32.const -1))
(assert_return (invoke "grow" (i32.const -1)) (i32.const -1))
(assert_return (invoke "grow" (i32.const 0)) (i32.const -2))
