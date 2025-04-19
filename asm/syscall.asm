
.globl sys_handler

sys_handler:
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rax

    mov rdi, rsp
    call sys_dispatch

    pop rax
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11

    sysretq
