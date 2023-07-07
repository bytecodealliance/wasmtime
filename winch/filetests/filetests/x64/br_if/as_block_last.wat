;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-block-last") (param i32)
    (block (call $dummy) (call $dummy) (br_if 0 (local.get 0)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 e800000000           	call	0x16
;;   16:	 e800000000           	call	0x1b
;;   1b:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   1f:	 85c9                 	test	ecx, ecx
;;   21:	 0f8500000000         	jne	0x27
;;   27:	 4883c410             	add	rsp, 0x10
;;   2b:	 5d                   	pop	rbp
;;   2c:	 c3                   	ret	
