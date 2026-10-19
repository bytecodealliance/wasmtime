;; A component which exports `wasi:http/handler` under the label `h`, as in
;; `export h: wasi:http/handler@0.3.0;`. `wasmtime serve` only looks for the
;; export named after the interface itself, and fails before looking at the
;; exported instance's contents, so it's left empty here.
(component
  (instance $h)
  (export "h" (implements "wasi:http/handler@0.3.0") (instance $h))
)
