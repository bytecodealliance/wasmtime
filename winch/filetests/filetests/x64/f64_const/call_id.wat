;;! target = "x86_64"

(module
  (func $id-f64 (param f64) (result f64) (local.get 0))
  (func (export "type-first-f64") (result f64) (call $id-f64 (f64.const 1.32)))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   18:	 4883c410             	add	rsp, 0x10
;;   1c:	 5d                   	pop	rbp
;;   1d:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883ec08             	sub	rsp, 8
;;   10:	 f20f100510000000     	movsd	xmm0, qword ptr [rip + 0x10]
;;   18:	 e800000000           	call	0x1d
;;   1d:	 4883c408             	add	rsp, 8
;;   21:	 4883c408             	add	rsp, 8
;;   25:	 5d                   	pop	rbp
;;   26:	 c3                   	ret	
;;   27:	 001f                 	add	byte ptr [rdi], bl
;;   29:	 85eb                 	test	ebx, ebp
;;   2b:	 51                   	push	rcx
