(module
    (type $ft (func))
    (type $ct (cont $ft))

    (func $entry (export "entry")
        (call $a)
    )

    (func $a (export "a")
        (resume $ct (cont.new $ct (ref.func $b)))
    )

    (func $b (export "b")
        (call $c)
    )

    (func $c (export "c")
        (resume $ct (cont.new $ct (ref.func $d)))
    )

    (func $d (export "d")
      (call $e)
    )

    (func $e (export "e")
        (resume $ct (cont.new $ct (ref.func $f)))
    )

    (func $f (export "f")
        (unreachable)
    )
)
