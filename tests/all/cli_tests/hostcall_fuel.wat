;; This is a test which asserts that it's not possible to get a >100x fuel
;; amplification when returning values to the host that are deserialized as a
;; dynamically typed `Val`. Each test here returns `(list $t)` where the list
;; has 10 elements, and each test should not blow the 1000 fuel limit meaning
;; that no one individual value can exceed 100 fuel.

(component

  (core module $m
    (memory (export "m") 1)

    (func (export "f1") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))
      i32.const 100
    )

    (func (export "f2") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))
      i32.const 100
    )

    (func (export "f3") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))

      (i32.store (i32.const 0x00) (i32.const 0x01010101))
      (i32.store (i32.const 0x04) (i32.const 0x01010101))
      (i32.store (i32.const 0x08) (i32.const 0x01010101))
      (i32.store (i32.const 0x0c) (i32.const 0x01010101))
      (i32.store (i32.const 0x10) (i32.const 0x01010101))
      (i32.store (i32.const 0x14) (i32.const 0x01010101))
      (i32.store (i32.const 0x18) (i32.const 0x01010101))
      (i32.store (i32.const 0x1c) (i32.const 0x01010101))
      (i32.store (i32.const 0x20) (i32.const 0x01010101))
      (i32.store (i32.const 0x24) (i32.const 0x01010101))

      i32.const 100
    )

    (func (export "f4") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))
      i32.const 100
    )

    (func (export "f5") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))
      i32.const 100
    )

    (func (export "f6") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))
      i32.const 100
    )

    (func (export "f7") (result i32)
      (i32.store (i32.const 100) (i32.const 0))
      (i32.store (i32.const 104) (i32.const 10))
      i32.const 100

      (i32.store8 (i32.const 0) (i32.const 0xff))
      (i32.store8 (i32.const 1) (i32.const 0xff))
      (i32.store8 (i32.const 2) (i32.const 0xff))
      (i32.store8 (i32.const 3) (i32.const 0xff))
      (i32.store8 (i32.const 4) (i32.const 0xff))
      (i32.store8 (i32.const 5) (i32.const 0xff))
      (i32.store8 (i32.const 6) (i32.const 0xff))
      (i32.store8 (i32.const 7) (i32.const 0xff))
      (i32.store8 (i32.const 8) (i32.const 0xff))
      (i32.store8 (i32.const 9) (i32.const 0xff))
    )
  )

  (core instance $i (instantiate $m))

  (type $t1' (record
    (field "f1" bool)
    (field "f2" bool)
    (field "f3" bool)
    (field "f4" bool)
    (field "f5" bool)
    (field "f6" bool)
    (field "f7" bool)
    (field "f8" bool)
    (field "f9" bool)
    (field "f10" bool)
  ))
  (export $t1 "t1" (type $t1'))
  (func (export "f1") (result (list $t1))
    (canon lift (core func $i "f1") (memory (core memory $i "m"))))

  (type $t2 (tuple bool bool bool bool bool bool bool bool bool bool))
  (func (export "f2") (result (list $t2))
    (canon lift (core func $i "f2") (memory (core memory $i "m"))))

  (type $t3 (option (option (option u8))))
  (func (export "f3") (result (list $t3))
    (canon lift (core func $i "f3") (memory (core memory $i "m"))))

  (type $t4' (variant (case "really-long-name-that-just-keeps-on-going-oh-boy-here-we-go-some-more")))
  (export $t4 "t4" (type $t4'))
  (func (export "f4") (result (list $t4))
    (canon lift (core func $i "f4") (memory (core memory $i "m"))))

  (type $t5' (enum "really-long-name-that-just-keeps-on-going-oh-boy-here-we-go-some-more"))
  (export $t5 "t5" (type $t5'))
  (func (export "f5") (result (list $t5))
    (canon lift (core func $i "f5") (memory (core memory $i "m"))))

  (type $t6' (record
    (field "really-long-name-that-just-keeps-on-going-oh-boy-here-we-go-some-more" bool)
  ))
  (export $t6 "t6" (type $t6'))
  (func (export "f6") (result (list $t6))
    (canon lift (core func $i "f6") (memory (core memory $i "m"))))

  (type $t7' (flags
    "this-is-a-really-long-flag1"
    "this-is-a-really-long-flag2"
    "this-is-a-really-long-flag3"
    "this-is-a-really-long-flag4"
    "this-is-a-really-long-flag5"
    "this-is-a-really-long-flag6"
    "this-is-a-really-long-flag7"
    "this-is-a-really-long-flag8"
  ))
  (export $t7 "t7" (type $t7'))
  (func (export "f7") (result (list $t7))
    (canon lift (core func $i "f7") (memory (core memory $i "m"))))
)
