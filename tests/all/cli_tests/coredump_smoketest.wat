(module
    (func (export "a")
        call $b
    )
    (func $b
        call $c
    )
    (func $c 
        unreachable
    )
)
