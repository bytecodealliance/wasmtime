;;! target = "x86_64"

(module
  (func (export "as-br_if-cond")
    (block (br_if 0 (unreachable)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 0f0b                 	ud2	
;;    e:	 4883c408             	add	rsp, 8
;;   12:	 5d                   	pop	rbp
;;   13:	 c3                   	ret	
