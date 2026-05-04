.globl sys_handler
sys_handler:
    push rsp
    push r15
    push r14
    push r13
    push r12
    push rbp
    push rbx
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push r11
    push rcx
    push rax

    mov rdi, rsp
    call sys_dispatch

    add rsp, 8

    pop rcx
    pop r11
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop rbx
    pop rbp
    pop r12
    pop r13
    pop r14
    pop r15
    pop rsp
    sysretq
