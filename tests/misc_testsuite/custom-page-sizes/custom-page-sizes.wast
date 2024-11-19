;;! custom_page_sizes = true
;;! multi_memory = true

;; Check all the valid custom page sizes.
(module (memory 1 (pagesize 1)))
(module (memory 1 (pagesize 65536)))

;; Check them all again with maximums specified.
(module (memory 1 2 (pagesize 1)))
(module (memory 1 2 (pagesize 65536)))

;; Check the behavior of memories with page size 1.
(module
  (memory 0 (pagesize 1))
  (func (export "size") (result i32)
    memory.size
  )
  (func (export "grow") (param i32) (result i32)
    (memory.grow (local.get 0))
  )
  (func (export "load") (param i32) (result i32)
    (i32.load8_u (local.get 0))
  )
  (func (export "store") (param i32 i32)
    (i32.store8 (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "size") (i32.const 0))
(assert_trap (invoke "load" (i32.const 0)) "out of bounds memory access")

(assert_return (invoke "grow" (i32.const 65536)) (i32.const 0))
(assert_return (invoke "size") (i32.const 65536))
(assert_return (invoke "load" (i32.const 65535)) (i32.const 0))
(assert_return (invoke "store" (i32.const 65535) (i32.const 1)))
(assert_return (invoke "load" (i32.const 65535)) (i32.const 1))
(assert_trap (invoke "load" (i32.const 65536)) "out of bounds memory access")

(assert_return (invoke "grow" (i32.const 65536)) (i32.const 65536))
(assert_return (invoke "size") (i32.const 131072))
(assert_return (invoke "load" (i32.const 131071)) (i32.const 0))
(assert_return (invoke "store" (i32.const 131071) (i32.const 1)))
(assert_return (invoke "load" (i32.const 131071)) (i32.const 1))
(assert_trap (invoke "load" (i32.const 131072)) "out of bounds memory access")

;; Although smaller page sizes let us get to memories larger than 2**16 pages,
;; we can't do that with the default page size, even if we explicitly state it
;; as a custom page size.
(module
  (memory 0 (pagesize 65536))
  (func (export "size") (result i32)
    memory.size
  )
  (func (export "grow") (param i32) (result i32)
    (memory.grow (local.get 0))
  )
)
(assert_return (invoke "size") (i32.const 0))
(assert_return (invoke "grow" (i32.const 65537)) (i32.const -1))
(assert_return (invoke "size") (i32.const 0))

;; Can copy between memories of different page sizes.
(module
  (memory $small 10 (pagesize 1))
  (memory $large 1 (pagesize 65536))

  (data (memory $small) (i32.const 0) "\11\22\33\44")
  (data (memory $large) (i32.const 0) "\55\66\77\88")

  (func (export "copy-small-to-large") (param i32 i32 i32)
    (memory.copy $large $small (local.get 0) (local.get 1) (local.get 2))
  )

  (func (export "copy-large-to-small") (param i32 i32 i32)
    (memory.copy $small $large (local.get 0) (local.get 1) (local.get 2))
  )

  (func (export "load8-small") (param i32) (result i32)
    (i32.load8_u $small (local.get 0))
  )

  (func (export "load8-large") (param i32) (result i32)
    (i32.load8_u $large (local.get 0))
  )
)

(assert_return (invoke "copy-small-to-large" (i32.const 6) (i32.const 0) (i32.const 2)))
(assert_return (invoke "load8-large" (i32.const 6)) (i32.const 0x11))
(assert_return (invoke "load8-large" (i32.const 7)) (i32.const 0x22))

(assert_return (invoke "copy-large-to-small" (i32.const 4) (i32.const 1) (i32.const 3)))
(assert_return (invoke "load8-small" (i32.const 4)) (i32.const 0x66))
(assert_return (invoke "load8-small" (i32.const 5)) (i32.const 0x77))
(assert_return (invoke "load8-small" (i32.const 6)) (i32.const 0x88))

;; Can link together modules that export and import memories with custom page
;; sizes.

(module $m
  (memory (export "small-pages-memory") 0 (pagesize 1))
  (memory (export "large-pages-memory") 0 (pagesize 65536))
)
(register "m" $m)

(module
  (memory (import "m" "small-pages-memory") 0 (pagesize 1))
)

(module
  (memory (import "m" "large-pages-memory") 0 (pagesize 65536))
)

(module
  (memory 8 8 (pagesize 0x1))
  (func (export "load64") (param i32) (result i64)
    local.get 0
    i64.load
  )
)

(assert_return (invoke "load64" (i32.const 0)) (i64.const 0))
