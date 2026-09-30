;;! component_model_async = true

;; A destructor run by `resource.drop` gets fresh context slots, and what it
;; stores there doesn't leak back out to whoever performed the drop.
(component
  (core func $get (canon context.get i32 0))
  (core func $set (canon context.set i32 0))

  (core module $Dtor
    (import "" "get" (func $get (result i32)))
    (import "" "set" (func $set (param i32)))
    (global $ran (mut i32) (i32.const 0))
    (func (export "dtor") (param i32)
      (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
      (call $set (i32.const 0xdead))
      (global.set $ran (i32.add (global.get $ran) (i32.const 1))))
    (func (export "ran") (result i32) (global.get $ran))
  )
  (core instance $dtor (instantiate $Dtor (with "" (instance
    (export "get" (func $get))
    (export "set" (func $set))))))

  (type $r (resource (rep i32) (dtor (core func $dtor "dtor"))))
  (core func $new (canon resource.new $r))
  (core func $drop (canon resource.drop $r))

  (core module $M
    (import "" "get" (func $get (result i32)))
    (import "" "set" (func $set (param i32)))
    (import "" "new" (func $new (param i32) (result i32)))
    (import "" "drop" (func $drop (param i32)))
    (import "" "ran" (func $ran (result i32)))
    (func (export "f")
      (call $set (i32.const 0x1234))
      (call $drop (call $new (i32.const 100)))

      ;; the destructor must have run, and must not have disturbed our slot
      (if (i32.ne (call $ran) (i32.const 1)) (then unreachable))
      (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable)))
  )
  (core instance $m (instantiate $M (with "" (instance
    (export "get" (func $get))
    (export "set" (func $set))
    (export "new" (func $new))
    (export "drop" (func $drop))
    (export "ran" (func $dtor "ran"))))))

  (func (export "f") (canon lift (core func $m "f")))
)
(assert_return (invoke "f"))

;; Same as above, but for a resource with no destructor at all: nothing runs, so
;; nothing may perturb the caller's context slots either.
(component
  (core func $get (canon context.get i32 0))
  (core func $set (canon context.set i32 0))

  (type $r (resource (rep i32)))
  (core func $new (canon resource.new $r))
  (core func $drop (canon resource.drop $r))

  (core module $M
    (import "" "get" (func $get (result i32)))
    (import "" "set" (func $set (param i32)))
    (import "" "new" (func $new (param i32) (result i32)))
    (import "" "drop" (func $drop (param i32)))
    (func (export "f")
      (call $set (i32.const 0x1234))
      (call $drop (call $new (i32.const 100)))
      (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable)))
  )
  (core instance $m (instantiate $M (with "" (instance
    (export "get" (func $get))
    (export "set" (func $set))
    (export "new" (func $new))
    (export "drop" (func $drop))))))

  (func (export "f") (canon lift (core func $m "f")))
)
(assert_return (invoke "f"))

;; Destructors nest: a destructor which itself drops a resource sees the inner
;; destructor start from zeroed slots and gets its own slots back afterwards.
(component
  (core func $get (canon context.get i32 0))
  (core func $set (canon context.set i32 0))

  ;; The innermost destructor.
  (core module $Dtor2
    (import "" "get" (func $get (result i32)))
    (import "" "set" (func $set (param i32)))
    (global $ran (mut i32) (i32.const 0))
    (func (export "dtor") (param i32)
      (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
      (call $set (i32.const 0x3333))
      (global.set $ran (i32.add (global.get $ran) (i32.const 1))))
    (func (export "ran") (result i32) (global.get $ran))
  )
  (core instance $dtor2 (instantiate $Dtor2 (with "" (instance
    (export "get" (func $get))
    (export "set" (func $set))))))
  (type $r2 (resource (rep i32) (dtor (core func $dtor2 "dtor"))))
  (core func $new2 (canon resource.new $r2))
  (core func $drop2 (canon resource.drop $r2))

  ;; The outer destructor, which itself drops an `$r2`.
  (core module $Dtor1
    (import "" "get" (func $get (result i32)))
    (import "" "set" (func $set (param i32)))
    (import "" "new2" (func $new2 (param i32) (result i32)))
    (import "" "drop2" (func $drop2 (param i32)))
    (global $ran (mut i32) (i32.const 0))
    (func (export "dtor") (param i32)
      (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
      (call $set (i32.const 0x2222))
      (call $drop2 (call $new2 (i32.const 200)))
      (if (i32.ne (call $get) (i32.const 0x2222)) (then unreachable))
      (global.set $ran (i32.add (global.get $ran) (i32.const 1))))
    (func (export "ran") (result i32) (global.get $ran))
  )
  (core instance $dtor1 (instantiate $Dtor1 (with "" (instance
    (export "get" (func $get))
    (export "set" (func $set))
    (export "new2" (func $new2))
    (export "drop2" (func $drop2))))))
  (type $r1 (resource (rep i32) (dtor (core func $dtor1 "dtor"))))
  (core func $new1 (canon resource.new $r1))
  (core func $drop1 (canon resource.drop $r1))

  (core module $M
    (import "" "get" (func $get (result i32)))
    (import "" "set" (func $set (param i32)))
    (import "" "new1" (func $new1 (param i32) (result i32)))
    (import "" "drop1" (func $drop1 (param i32)))
    (import "" "ran1" (func $ran1 (result i32)))
    (import "" "ran2" (func $ran2 (result i32)))
    (func (export "f")
      (call $set (i32.const 0x1111))
      (call $drop1 (call $new1 (i32.const 100)))
      (if (i32.ne (call $ran1) (i32.const 1)) (then unreachable))
      (if (i32.ne (call $ran2) (i32.const 1)) (then unreachable))
      (if (i32.ne (call $get) (i32.const 0x1111)) (then unreachable)))
  )
  (core instance $m (instantiate $M (with "" (instance
    (export "get" (func $get))
    (export "set" (func $set))
    (export "new1" (func $new1))
    (export "drop1" (func $drop1))
    (export "ran1" (func $dtor1 "ran"))
    (export "ran2" (func $dtor2 "ran"))))))

  (func (export "f") (canon lift (core func $m "f")))
)
(assert_return (invoke "f"))

;; Component composition case: `$B` drops a handle owned by `$A`, and `$A`'s
;; destructor must still start with zeroed slots without perturbing `$B`'s.
(component
  (component $A
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))

    (core module $Dtor
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (global $ran (mut i32) (i32.const 0))
      (func (export "dtor") (param i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0xdead))
        (global.set $ran (i32.add (global.get $ran) (i32.const 1))))
      (func (export "ran") (result i32) (global.get $ran))
    )
    (core instance $dtor (instantiate $Dtor (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))

    (type $t (resource (rep i32) (dtor (core func $dtor "dtor"))))
    (core func $new (canon resource.new $t))

    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (import "" "new" (func $new (param i32) (result i32)))
      ;; note that this scribbles over its own task's slots on the way out to
      ;; make sure that nothing of it survives into the destructor's task
      (func (export "make") (result i32) (local $r i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (local.set $r (call $new (i32.const 100)))
        (call $set (i32.const 0x5555))
        (local.get $r))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))
      (export "new" (func $new))))))

    (export $t' "t" (type $t))
    (func (export "make") (result (own $t')) (canon lift (core func $m "make")))
    (func (export "ran") (result u32) (canon lift (core func $dtor "ran")))
  )

  (component $B
    (import "a" (instance $a
      (export "t" (type $t (sub resource)))
      (export "make" (func (result (own $t))))
      (export "ran" (func (result u32)))))

    (core func $make (canon lower (func $a "make")))
    (core func $ran (canon lower (func $a "ran")))
    (alias export $a "t" (type $t))
    (core func $drop (canon resource.drop $t))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))

    (core module $N
      (import "" "make" (func $make (result i32)))
      (import "" "ran" (func $ran (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "run") (local $r i32)
        (local.set $r (call $make))
        (call $set (i32.const 0x1234))
        (call $drop (local.get $r))
        (if (i32.ne (call $ran) (i32.const 1)) (then unreachable))
        (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable)))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "make" (func $make))
      (export "ran" (func $ran))
      (export "drop" (func $drop))
      (export "get" (func $get))
      (export "set" (func $set))))))

    (func (export "run") (canon lift (core func $n "run")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
(assert_return (invoke "run"))

;; As above, but the destructor forces its deferred thread into a real one with
;; a guest->host call partway through, so `resource.drop` takes the out-of-line
;; exit path. The destructor's own slots must survive that, and `$B`'s must
;; still come back afterwards.
(component
  (import "wasmtime" (instance $wasmtime (export "gc" (func))))

  (component $A
    (import "poke" (func $poke))
    (core func $poke' (canon lower (func $poke)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))

    (core module $Dtor
      (import "" "poke" (func $poke'))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (global $ran (mut i32) (i32.const 0))
      (func (export "dtor") (param i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0x0dead000))
        ;; Force the deferred thread pushed by `resource.drop`.
        (call $poke')
        ;; Our own slots survive the force.
        (if (i32.ne (call $get) (i32.const 0x0dead000)) (then unreachable))
        (global.set $ran (i32.add (global.get $ran) (i32.const 1))))
      (func (export "ran") (result i32) (global.get $ran)))
    (core instance $dtor (instantiate $Dtor (with "" (instance
      (export "poke" (func $poke'))
      (export "get" (func $get))
      (export "set" (func $set))))))

    (type $t (resource (rep i32) (dtor (core func $dtor "dtor"))))
    (core func $new (canon resource.new $t))
    (core module $M
      (import "" "new" (func $new (param i32) (result i32)))
      (func (export "make") (result i32) (call $new (i32.const 100))))
    (core instance $m (instantiate $M (with "" (instance
      (export "new" (func $new))))))

    (export $t' "t" (type $t))
    (func (export "make") (result (own $t')) (canon lift (core func $m "make")))
    (func (export "ran") (result u32) (canon lift (core func $dtor "ran"))))

  (component $B
    (import "a" (instance $a
      (export "t" (type $t (sub resource)))
      (export "make" (func (result (own $t))))
      (export "ran" (func (result u32)))))
    (core func $make (canon lower (func $a "make")))
    (core func $ran (canon lower (func $a "ran")))
    (alias export $a "t" (type $t))
    (core func $drop (canon resource.drop $t))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "make" (func $make (result i32)))
      (import "" "ran" (func $ran (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "run") (result i32) (local $r i32)
        (local.set $r (call $make))
        (call $set (i32.const 0x1234))
        (call $drop (local.get $r))
        ;; Restored after the destructor forced the slow exit path.
        (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable))
        (call $ran)))
    (core instance $n (instantiate $N (with "" (instance
      (export "make" (func $make))
      (export "ran" (func $ran))
      (export "drop" (func $drop))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "run") (result u32) (canon lift (core func $n "run"))))

  (instance $a (instantiate $A (with "poke" (func $wasmtime "gc"))))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(assert_return (invoke "run") (u32.const 1))
(assert_return (invoke "run") (u32.const 2))
(assert_return (invoke "run") (u32.const 3))
(assert_return (invoke "run") (u32.const 4))
