;;! target = "x86_64"
(module
  (func (export "as-br-if-cond")
    (block (br_if 0 (br_if 0 (i32.const 1) (i32.const 1))))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b901000000           	mov	ecx, 1
;;   11:	 85c9                 	test	ecx, ecx
;;   13:	 0f850d000000         	jne	0x26
;;   19:	 b901000000           	mov	ecx, 1
;;   1e:	 85c9                 	test	ecx, ecx
;;   20:	 0f8500000000         	jne	0x26
;;   26:	 4883c408             	add	rsp, 8
;;   2a:	 5d                   	pop	rbp
;;   2b:	 c3                   	ret	
