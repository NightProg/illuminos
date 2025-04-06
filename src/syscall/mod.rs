use alloc::vec::Vec;
use x86_64::structures::idt::InterruptStackFrame;
use core::arch::asm;
use crate::io::port::Fd;

pub const SYSCALL_WRITE: u64 = 0;


#[derive(Debug, Clone, Copy)]
pub struct Stack {
    pub rsp: u64,
    pub rbp: u64,
}

impl Stack {
    pub fn load() -> Self {
        let mut stack = Stack::new(0, 0);
        unsafe {
            asm!("mov {}, rsp", out(reg) stack.rsp);
            asm!("mov {}, rbp", out(reg) stack.rbp);
        }
        stack
    }
    pub fn new(rsp: u64, rbp: u64) -> Self {
        Stack { rsp, rbp }
    }

    pub fn read_u64(&self, n: usize) -> u64 {
        let mut value: u64 = 0;
        let offset = n * 8;
        unsafe {
            asm!("mov {}, [{} + {}]", out(reg) value, in(reg) self.rsp, in(reg) offset);
            asm!("sub rsp, {}", in(reg) offset);
        }
        value
    }

    pub fn write_i64(&mut self, value: i64) {
        unsafe {
            asm!("mov [{} + 8], {}", in(reg) self.rsp, in(reg) value);
            asm!("add rsp, 8");
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Registers {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

impl Registers {
    pub fn rax() -> u64 {
        let rax: u64;
        unsafe {
            asm!("mov {}, rax", out(reg) rax);
        }
        rax
    }

    pub fn rbx() -> u64 {
        let rbx: u64;
        unsafe {
            asm!("mov {}, rbx", out(reg) rbx);
        }
        rbx
    }

    pub fn rcx() -> u64 {
        let rcx: u64;
        unsafe {
            asm!("mov {}, rcx", out(reg) rcx);
        }
        rcx
    }

    pub fn rdx() -> u64 {
        let rdx: u64;
        unsafe {
            asm!("mov {}, rdx", out(reg) rdx);
        }
        rdx
    }

    pub fn rsi() -> u64 {
        let rsi: u64;
        unsafe {
            asm!("mov {}, rsi", out(reg) rsi);
        }
        rsi
    }

    pub fn rdi() -> u64 {
        let rdi: u64;
        unsafe {
            asm!("mov {}, rdi", out(reg) rdi);
        }
        rdi
    }

    pub fn r8() -> u64 {
        let r8: u64;
        unsafe {
            asm!("mov {}, r8", out(reg) r8);
        }
        r8
    }

    pub fn r9() -> u64 {
        let r9: u64;
        unsafe {
            asm!("mov {}, r9", out(reg) r9);
        }
        r9
    }

    pub fn r10() -> u64 {
        let r10: u64;
        unsafe {
            asm!("mov {}, r10", out(reg) r10);
        }
        r10
    }

    pub fn r11() -> u64 {
        let r11: u64;
        unsafe {
            asm!("mov {}, r11", out(reg) r11);
        }
        r11
    }

    pub fn r12() -> u64 {
        let r12: u64;
        unsafe {
            asm!("mov {}, r12", out(reg) r12);
        }
        r12
    }

    pub fn r13() -> u64 {
        let r13: u64;
        unsafe {
            asm!("mov {}, r13", out(reg) r13);
        }
        r13
    }

    pub fn r14() -> u64 {
        let r14: u64;
        unsafe {
            asm!("mov {}, r14", out(reg) r14);
        }
        r14
    }

    pub fn r15() -> u64 {
        let r15: u64;
        unsafe {
            asm!("mov {}, r15", out(reg) r15);
        }
        r15
    }

    pub fn save() -> Self {
        Self {
            rax: Registers::rax(),
            rbx: Registers::rbx(),
            rcx: Registers::rcx(),
            rdx: Registers::rdx(),
            rsi: Registers::rsi(),
            rdi: Registers::rdi(),
            r8: Registers::r8(),
            r9: Registers::r9(),
            r10: Registers::r10(),
            r11: Registers::r11(),
            r12: Registers::r12(),
            r13: Registers::r13(),
            r14: Registers::r14(),
            r15: Registers::r15(),
        }
    }
}

pub struct Args<T: CallConvention> {
    _marker: core::marker::PhantomData<T>,
    registers: Registers,
    stack: Stack,
}

impl<T: CallConvention> Args<T> {
    pub fn new(stack: Stack, regs: Registers) -> Self {
        Self {
            _marker: core::marker::PhantomData,
            registers: regs,
            stack,
        }
    }

    pub fn arg(&self, arg: usize) -> u64 {
        T::arg(self.stack, self.registers, arg)
    }

    pub fn args(&self, arg_length: usize) -> Vec<u64> {
        T::args(self.stack, self.registers, arg_length)
    }
}

pub trait CallConvention {
    fn arg(stack: Stack, regs: Registers, arg: usize) -> u64;
    fn args(stack: Stack, regs: Registers, arg_length: usize) -> Vec<u64> {
        let mut args = Vec::new();
        for i in 0..arg_length {
            args.push(Self::arg(stack, regs, i));
        }
        args
    }
}

pub struct GccCallConvention;

impl CallConvention for GccCallConvention {
    fn arg(stack: Stack, regs: Registers, arg: usize) -> u64 {
        match arg {
            0 => regs.rdi,
            1 => regs.rsi,
            2 => regs.rdx,
            3 => regs.rcx,
            4 => regs.r8,
            5 => regs.r9,

            n => stack.read_u64(n - 5),
        }
    }
}

pub trait SyscallHandler<T: CallConvention> {
    fn handle(&self, _stack_frame: InterruptStackFrame, args: Args<T>);
}

pub struct SyscallWrite;

impl<T: CallConvention> SyscallHandler<T> for SyscallWrite {
    fn handle(&self, _stack_frame: InterruptStackFrame, args: Args<T>) {
        let mut fd = Fd(Args::<T>::arg(&args, 0) as usize);

        let buf = Args::<T>::arg(&args, 1);
        let count = Args::<T>::arg(&args, 2);

        let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, count as usize) };

        fd.write(slice);
    }
}
