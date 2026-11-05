(module
  (memory (export "memory") 1)
  (func (export "_start")
    ;; Just below the watched range [8, 12).
    (i32.store8 (i32.const 7) (i32.const 1))
    ;; Partially overlaps the watched range.
    (i32.store offset=6 (i32.const 0) (i32.const 0x11223344))
    ;; Just above the watched range.
    (i32.store8 (i32.const 12) (i32.const 2))
    ;; Covers the whole watched range.
    (memory.fill (i32.const 0) (i32.const 0xaa) (i32.const 16))
  )
)
