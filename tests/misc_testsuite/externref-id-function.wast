;;! reference_types = true

(module
  (func (export "identity") (param externref) (result externref)
    local.get 0))

(assert_return (invoke "identity" (ref.null extern))
               (ref.null extern))
(assert_return (invoke "identity" (ref.extern 1))
               (ref.extern 1))
