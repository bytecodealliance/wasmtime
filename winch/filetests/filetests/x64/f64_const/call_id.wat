;;! target = "x86_64"

(module
  (func $id-f64 (param f64) (result f64) (local.get 0))
  (func (export "type-first-f64") (result f64) (call $id-f64 (f64.const 1.32)))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f100510000000     	movsd	xmm0, qword ptr [rip + 0x10]
;;      	 e800000000           	call	0x1d
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 001f                 	add	byte ptr [rdi], bl
;;   29:	 85eb                 	test	ebx, ebp
;;   2b:	 51                   	push	rcx
