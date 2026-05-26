;;! gc = true

;; Basic copy of i32 elements between two arrays.
(module
  (type $arr (array (mut i32)))

  (func (export "basic-copy") (result i32)
    (local $src (ref $arr))
    (local $dst (ref $arr))

    ;; Source: [10, 20, 30, 40, 50]
    (local.set $src (array.new_fixed $arr 5
      (i32.const 10)
      (i32.const 20)
      (i32.const 30)
      (i32.const 40)
      (i32.const 50)
    ))

    ;; Destination: [0, 0, 0, 0, 0]
    (local.set $dst (array.new $arr (i32.const 0) (i32.const 5)))

    ;; Copy src[1..4] to dst[0..3]
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (local.get $src) (i32.const 1)
      (i32.const 3)
    )

    ;; dst should be [20, 30, 40, 0, 0]
    (i32.add
      (i32.add
        (array.get $arr (local.get $dst) (i32.const 0))
        (array.get $arr (local.get $dst) (i32.const 1))
      )
      (i32.add
        (array.get $arr (local.get $dst) (i32.const 2))
        (array.get $arr (local.get $dst) (i32.const 3))
      )
    )
  )
)

;; 20 + 30 + 40 + 0 = 90
(assert_return (invoke "basic-copy") (i32.const 90))

;; Overlapping copy within same array: forward direction (src > dst).
(module
  (type $arr (array (mut i32)))

  (func (export "overlap-forward") (result i32)
    (local $a (ref $arr))

    ;; [10, 20, 30, 40, 50]
    (local.set $a (array.new_fixed $arr 5
      (i32.const 10)
      (i32.const 20)
      (i32.const 30)
      (i32.const 40)
      (i32.const 50)
    ))

    ;; Copy a[2..5] to a[0..3] (src_index=2 > dst_index=0)
    (array.copy $arr $arr
      (local.get $a) (i32.const 0)
      (local.get $a) (i32.const 2)
      (i32.const 3)
    )

    ;; a should be [30, 40, 50, 40, 50]
    (i32.add
      (i32.add
        (array.get $arr (local.get $a) (i32.const 0))
        (array.get $arr (local.get $a) (i32.const 1))
      )
      (i32.add
        (array.get $arr (local.get $a) (i32.const 2))
        (array.get $arr (local.get $a) (i32.const 3))
      )
    )
  )
)

;; 30 + 40 + 50 + 40 = 160
(assert_return (invoke "overlap-forward") (i32.const 160))

;; Overlapping copy within same array: backward direction (dst > src).
(module
  (type $arr (array (mut i32)))

  (func (export "overlap-backward") (result i32)
    (local $a (ref $arr))

    ;; [10, 20, 30, 40, 50]
    (local.set $a (array.new_fixed $arr 5
      (i32.const 10)
      (i32.const 20)
      (i32.const 30)
      (i32.const 40)
      (i32.const 50)
    ))

    ;; Copy a[0..3] to a[2..5] (src_index=0 < dst_index=2)
    (array.copy $arr $arr
      (local.get $a) (i32.const 2)
      (local.get $a) (i32.const 0)
      (i32.const 3)
    )

    ;; a should be [10, 20, 10, 20, 30]
    (i32.add
      (i32.add
        (array.get $arr (local.get $a) (i32.const 0))
        (array.get $arr (local.get $a) (i32.const 1))
      )
      (i32.add
        (array.get $arr (local.get $a) (i32.const 2))
        (array.get $arr (local.get $a) (i32.const 3))
      )
    )
  )
)

;; 10 + 20 + 10 + 20 = 60
(assert_return (invoke "overlap-backward") (i32.const 60))

;; Zero-length copy is a no-op.
(module
  (type $arr (array (mut i32)))

  (func (export "zero-length") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_fixed $arr 3
      (i32.const 1)
      (i32.const 2)
      (i32.const 3)
    ))
    ;; Copy length 0 at array boundary -- should not trap.
    (array.copy $arr $arr
      (local.get $a) (i32.const 3)
      (local.get $a) (i32.const 3)
      (i32.const 0)
    )
    (array.get $arr (local.get $a) (i32.const 2))
  )
)

(assert_return (invoke "zero-length") (i32.const 3))

;; Copy of i8 (packed) elements.
(module
  (type $arr (array (mut i8)))

  (func (export "i8-copy") (result i32)
    (local $src (ref $arr))
    (local $dst (ref $arr))
    (local.set $src (array.new_fixed $arr 3
      (i32.const 100)
      (i32.const 200)
      (i32.const 42)
    ))
    (local.set $dst (array.new $arr (i32.const 0) (i32.const 3)))
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (local.get $src) (i32.const 0)
      (i32.const 3)
    )
    (i32.add
      (array.get_u $arr (local.get $dst) (i32.const 0))
      (array.get_u $arr (local.get $dst) (i32.const 2))
    )
  )
)

;; 100 + 42 = 142
(assert_return (invoke "i8-copy") (i32.const 142))

;; Out-of-bounds destination traps.
(module
  (type $arr (array (mut i32)))

  (func (export "oob-dst")
    (local $src (ref $arr))
    (local $dst (ref $arr))
    (local.set $src (array.new $arr (i32.const 0) (i32.const 5)))
    (local.set $dst (array.new $arr (i32.const 0) (i32.const 3)))
    ;; dst has length 3, copying 4 elements starting at 0 is out of bounds.
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (local.get $src) (i32.const 0)
      (i32.const 4)
    )
  )
)

(assert_trap (invoke "oob-dst") "out of bounds array access")

;; Out-of-bounds source traps.
(module
  (type $arr (array (mut i32)))

  (func (export "oob-src")
    (local $src (ref $arr))
    (local $dst (ref $arr))
    (local.set $src (array.new $arr (i32.const 0) (i32.const 3)))
    (local.set $dst (array.new $arr (i32.const 0) (i32.const 5)))
    ;; src has length 3, copying 4 elements starting at 0 is out of bounds.
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (local.get $src) (i32.const 0)
      (i32.const 4)
    )
  )
)

(assert_trap (invoke "oob-src") "out of bounds array access")

;; Null destination traps.
(module
  (type $arr (array (mut i32)))

  (func (export "null-dst")
    (local $src (ref $arr))
    (local.set $src (array.new $arr (i32.const 0) (i32.const 3)))
    (array.copy $arr $arr
      (ref.null $arr) (i32.const 0)
      (local.get $src) (i32.const 0)
      (i32.const 1)
    )
  )
)

(assert_trap (invoke "null-dst") "null reference")

;; Null source traps.
(module
  (type $arr (array (mut i32)))

  (func (export "null-src")
    (local $dst (ref $arr))
    (local.set $dst (array.new $arr (i32.const 0) (i32.const 3)))
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (ref.null $arr) (i32.const 0)
      (i32.const 1)
    )
  )
)

(assert_trap (invoke "null-src") "null reference")
