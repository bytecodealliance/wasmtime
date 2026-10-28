;;! component_model_map = true

(component
  (type (map u32 string))
  (type (map string u32))
  (type (map u32 u32))
  (type (map string (map u32 string)))
)

;; Every type in the `keytype` grammar can be used as a map key.
(component
  (type (map bool u32))
  (type (map s8 u32))
  (type (map u8 u32))
  (type (map s16 u32))
  (type (map u16 u32))
  (type (map s32 u32))
  (type (map u32 u32))
  (type (map s64 u32))
  (type (map u64 u32))
  (type (map char u32))
  (type (map string u32))
)

;; Any other type is rejected as a map key.
(assert_invalid
  (component (type (map f32 u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map f64 u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map (list u32) u32)))
  "invalid map key type")

(assert_invalid
  (component
    (type $r (record (field "a" u32)))
    (type (map $r u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map (option u32) u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map (tuple u32 u32) u32)))
  "invalid map key type")

(assert_invalid
  (component
    (type $t (resource (rep i32)))
    (type (map (own $t) u32)))
  "invalid map key type")

(assert_invalid
  (component
    (type $t (resource (rep i32)))
    (type (map (borrow $t) u32)))
  "invalid map key type")

;; Map values keep every entry, in order, including duplicate keys, both when
;; lifted from guest memory and when lowered into it.
(component
  (core module $m
    (memory (export "memory") 1)

    ;; Returns (ptr = 16, len = 3) pointing at the entries
    ;; (1, 10) (1, 20) (2, 30).
    (data (i32.const 8) "\10\00\00\00\03\00\00\00")
    (data (i32.const 16)
      "\01\00\00\00\0a\00\00\00"
      "\01\00\00\00\14\00\00\00"
      "\02\00\00\00\1e\00\00\00")
    (func (export "dups") (result i32) i32.const 8)

    ;; Returns the map it was given.
    (func (export "echo") (param i32 i32) (result i32)
      (i32.store (i32.const 0) (local.get 0))
      (i32.store (i32.const 4) (local.get 1))
      i32.const 0)

    ;; Bump allocator for the lowered argument, after the static data.
    (global $next (mut i32) (i32.const 64))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $ret i32)
      (local.set $ret (global.get $next))
      (global.set $next (i32.add (local.get $ret) (local.get 3)))
      (local.get $ret))
  )
  (core instance $i (instantiate $m))

  (type $t (map u32 u32))
  (func (export "dups") (result $t)
    (canon lift (core func $i "dups") (memory (core memory $i "memory"))))
  (func (export "echo") (param "m" $t) (result $t)
    (canon lift (core func $i "echo") (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))))
)

(assert_return (invoke "dups")
  (map.const
    (entry (u32.const 1) (u32.const 10))
    (entry (u32.const 1) (u32.const 20))
    (entry (u32.const 2) (u32.const 30))))

(assert_return
  (invoke "echo"
    (map.const
      (entry (u32.const 2) (u32.const 1))
      (entry (u32.const 1) (u32.const 2))
      (entry (u32.const 2) (u32.const 3))))
  (map.const
    (entry (u32.const 2) (u32.const 1))
    (entry (u32.const 1) (u32.const 2))
    (entry (u32.const 2) (u32.const 3))))
