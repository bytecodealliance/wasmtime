(module
  (type $arr_i8 (array i8))
  (type $arr_i64 (array i64))

  ;; Overflow on `elems_size = len * sizeof(elem_type)`
  (func (export "overflow-elems-size") (result (ref $arr_i64))
    (array.new_default $arr_i64 (i32.const -1))
  )

  ;; Overflow on `base_size + elems_size`
  (func (export "overflow-add-base-size") (result (ref $arr_i8))
    (array.new_default $arr_i8 (i32.const -1))
  )

  ;; Larger than can fit in `VMGcHeader`'s reserved 26 bits.
  (func (export "bigger-than-reserved-bits") (result (ref $arr_i8))
    (array.new_default $arr_i8 (i32.shl (i32.const 1) (i32.const 26)))
  )
)

(assert_trap (invoke "overflow-elems-size") "allocation size too large")
(assert_trap (invoke "overflow-add-base-size") "allocation size too large")
(assert_trap (invoke "bigger-than-reserved-bits") "allocation size too large")
