;;! gc = true
;;! simd = true

;; A small, constant-length `array.copy` is expanded inline as wide loads then
;; stores (every byte loaded before any is stored) instead of calling the
;; `memory_copy` libcall; the codegen shape is locked by
;; tests/disas/array-copy-inline.wat. These tests check that the inline path
;; produces correct results for every element width, that the threshold boundary
;; agrees (inline through 128 bytes, libcall above), and that overlapping copies
;; keep `memmove` semantics. The wast harness runs these across all collectors.

;; --- one inline copy per element size; check the first and last copied
;; --- element (encoded as last*1000 + first) so a wrong element-size stride is
;; --- caught.

(module
  (type $a (array (mut i8)))
  (func (export "i8") (result i32)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 8
      (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4)
      (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8)))
    (local.set $d (array.new_default $a (i32.const 8)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 8))
    (i32.add (i32.mul (array.get_u $a (local.get $d) (i32.const 7)) (i32.const 1000))
             (array.get_u $a (local.get $d) (i32.const 0)))))
(assert_return (invoke "i8") (i32.const 8001))

(module
  (type $a (array (mut i16)))
  (func (export "i16") (result i32)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 8
      (i32.const 10) (i32.const 20) (i32.const 30) (i32.const 40)
      (i32.const 50) (i32.const 60) (i32.const 70) (i32.const 80)))
    (local.set $d (array.new_default $a (i32.const 8)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 8))
    (i32.add (i32.mul (array.get_u $a (local.get $d) (i32.const 7)) (i32.const 1000))
             (array.get_u $a (local.get $d) (i32.const 0)))))
(assert_return (invoke "i16") (i32.const 80010))

(module
  (type $a (array (mut i32)))
  (func (export "i32") (result i32)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 8
      (i32.const 100) (i32.const 200) (i32.const 300) (i32.const 400)
      (i32.const 500) (i32.const 600) (i32.const 700) (i32.const 800)))
    (local.set $d (array.new_default $a (i32.const 8)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 8))
    (i32.add (i32.mul (array.get $a (local.get $d) (i32.const 7)) (i32.const 1000))
             (array.get $a (local.get $d) (i32.const 0)))))
(assert_return (invoke "i32") (i32.const 800100))

(module
  (type $a (array (mut i64)))
  (func (export "i64") (result i64)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 8
      (i64.const 1) (i64.const 2) (i64.const 3) (i64.const 4)
      (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 0x1_0000_0000)))
    (local.set $d (array.new_default $a (i32.const 8)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 8))
    (i64.add (array.get $a (local.get $d) (i32.const 7))
             (array.get $a (local.get $d) (i32.const 0)))))
(assert_return (invoke "i64") (i64.const 0x1_0000_0001))

(module
  (type $a (array (mut f32)))
  (func (export "f32") (result f32)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 4
      (f32.const 1.5) (f32.const 2.5) (f32.const 3.5) (f32.const 4.5)))
    (local.set $d (array.new_default $a (i32.const 4)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 4))
    (f32.add (array.get $a (local.get $d) (i32.const 0))
             (array.get $a (local.get $d) (i32.const 3)))))
(assert_return (invoke "f32") (f32.const 6.0))

(module
  (type $a (array (mut f64)))
  (func (export "f64") (result f64)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 4
      (f64.const 1.25) (f64.const 2.25) (f64.const 3.25) (f64.const 4.25)))
    (local.set $d (array.new_default $a (i32.const 4)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 4))
    (f64.add (array.get $a (local.get $d) (i32.const 0))
             (array.get $a (local.get $d) (i32.const 3)))))
(assert_return (invoke "f64") (f64.const 5.5))

(module
  (type $a (array (mut v128)))
  (func (export "v128") (result i32)
    (local $s (ref $a)) (local $d (ref $a))
    (local.set $s (array.new_fixed $a 2
      (i32x4.splat (i32.const 7)) (i32x4.splat (i32.const 9))))
    (local.set $d (array.new_default $a (i32.const 2)))
    (array.copy $a $a (local.get $d) (i32.const 0) (local.get $s) (i32.const 0) (i32.const 2))
    (i32.add (i32x4.extract_lane 0 (array.get $a (local.get $d) (i32.const 0)))
             (i32x4.extract_lane 0 (array.get $a (local.get $d) (i32.const 1))))))
(assert_return (invoke "v128") (i32.const 16))

;; --- threshold boundary: 16 i64 elements (128 bytes) is inlined, 17 (136 bytes)
;; --- falls back to the libcall; both must copy correctly.

(module
  (type $a (array (mut i64)))
  (func $sum (param $arr (ref $a)) (param $n i32) (result i64)
    (local $i i32) (local $acc i64)
    (block $e (loop $l
      (br_if $e (i32.ge_u (local.get $i) (local.get $n)))
      (local.set $acc (i64.add (local.get $acc)
        (array.get $a (local.get $arr) (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $l)))
    (local.get $acc))

  (func $src (result (ref $a))
    (array.new_fixed $a 17
      (i64.const 1) (i64.const 2) (i64.const 3) (i64.const 4) (i64.const 5)
      (i64.const 6) (i64.const 7) (i64.const 8) (i64.const 9) (i64.const 10)
      (i64.const 11) (i64.const 12) (i64.const 13) (i64.const 14) (i64.const 15)
      (i64.const 16) (i64.const 17)))

  ;; constant length 16 (128 bytes) -> inline
  (func (export "boundary-128") (result i64)
    (local $d (ref $a))
    (local.set $d (array.new_default $a (i32.const 17)))
    (array.copy $a $a (local.get $d) (i32.const 0) (call $src) (i32.const 0) (i32.const 16))
    (call $sum (local.get $d) (i32.const 17)))  ;; 1..16 copied, [16]=0 -> 136

  ;; constant length 17 (136 bytes) -> libcall
  (func (export "boundary-136") (result i64)
    (local $d (ref $a))
    (local.set $d (array.new_default $a (i32.const 17)))
    (array.copy $a $a (local.get $d) (i32.const 0) (call $src) (i32.const 0) (i32.const 17))
    (call $sum (local.get $d) (i32.const 17))))  ;; 1..17 -> 153
(assert_return (invoke "boundary-128") (i64.const 136))
(assert_return (invoke "boundary-136") (i64.const 153))

;; --- overlapping inline copy keeps memmove semantics for a wider element type
;; --- (i64), in both directions.

(module
  (type $a (array (mut i64)))
  (func $mk (result (ref $a))
    (array.new_fixed $a 5
      (i64.const 1) (i64.const 2) (i64.const 3) (i64.const 4) (i64.const 5)))

  ;; dst < src: copies a[2..5] over a[0..3] -> [3,4,5,4,5]
  (func (export "overlap-forward") (result i64)
    (local $a (ref $a))
    (local.set $a (call $mk))
    (array.copy $a $a (local.get $a) (i32.const 0) (local.get $a) (i32.const 2) (i32.const 3))
    (i64.add (i64.mul (array.get $a (local.get $a) (i32.const 0)) (i64.const 100))
             (array.get $a (local.get $a) (i32.const 2))))  ;; 3*100 + 5 = 305

  ;; dst > src: copies a[0..3] over a[2..5] -> [1,2,1,2,3]
  (func (export "overlap-backward") (result i64)
    (local $a (ref $a))
    (local.set $a (call $mk))
    (array.copy $a $a (local.get $a) (i32.const 2) (local.get $a) (i32.const 0) (i32.const 3))
    (i64.add (i64.mul (array.get $a (local.get $a) (i32.const 2)) (i64.const 100))
             (array.get $a (local.get $a) (i32.const 4)))))  ;; 1*100 + 3 = 103
(assert_return (invoke "overlap-forward") (i64.const 305))
(assert_return (invoke "overlap-backward") (i64.const 103))
