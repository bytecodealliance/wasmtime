;;! target = "x86_64"

(module
  (func (export "as-loop-first") (result i32)
    (block (result i32) (loop (result i32) (br 1 (i32.const 3)) (i32.const 2)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b803000000           	mov	eax, 3
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
