;; A compile-time builtin implementing the `host-api-a` import of
;; `compile-time-builtin-main.wat`.
;;
;; Note that the file stem is the same as the import name it satisfies, so that
;; `-C compile-time-builtin=path/to/host-api-a.wat` works without an explicit
;; name.
(component
  (core module $m
    (func (export "get") (result i32)
      i32.const 42)
  )
  (core instance $i (instantiate $m))
  (func (export "get") (result u32)
    (canon lift (core func $i "get")))
)
