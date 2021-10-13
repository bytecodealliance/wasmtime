;; This test contains the changes in
;; https://github.com/WebAssembly/reference-types/pull/104, and can be deleted
;; once that merges and we update our upstream tests.

(module
  (global $mr (mut externref) (ref.null extern))
  (func (export "get-mr") (result externref) (global.get $mr))
  (func (export "set-mr") (param externref) (global.set $mr (local.get 0)))
)

(assert_return (invoke "get-mr") (ref.null extern))
(assert_return (invoke "set-mr" (ref.extern 10)))
(assert_return (invoke "get-mr") (ref.extern 10))
