;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-loop-last") (param i32)
    (loop (call $dummy) (br_if 1 (local.get 0)))
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
;;   16:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   1a:	 85c9                 	test	ecx, ecx
;;   1c:	 0f8500000000         	jne	0x22
;;   22:	 4883c410             	add	rsp, 0x10
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	
