(module
    (func $hello (import "global" "hello"))
    (func (export "run") (call $hello))
)
