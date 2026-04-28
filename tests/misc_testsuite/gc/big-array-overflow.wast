;;! gc = true

(module
  (type $arr (array (mut i8)))
  (func (export "test")
    ;; With the copying collector, this allocation has
    ;;
    ;;     base_size=20,
    ;;     elem_size=1,
    ;;     length=4294967275
    ;;     total size = 20 + 4294967275 = 4294967295 = u32::MAX
    ;;
    ;; In alloc_raw, the rounding (size + 15) & !15 overflows u32
    (drop (array.new_default $arr (i32.const -21)))
  )
)

(assert_trap (invoke "test") "allocation size too large")
