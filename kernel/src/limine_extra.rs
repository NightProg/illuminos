#[repr(C)]
pub struct LimineKernelFileRequest {
    pub id: [u64; 4],
    pub rev: u64,
    pub res: *mut LimineKernelFileResponse,
}

unsafe impl Send for LimineKernelFileRequest {}
unsafe impl Sync for LimineKernelFileRequest {}

impl LimineKernelFileRequest {
    pub const fn new() -> Self {
        LimineKernelFileRequest {
            id: [
                0xc7b1dd30df4c8b88,
                0x0a82e883a194f07b,
                0xad97e90e83f1ed67,
                0x31eb5d1c5ff23b69,
            ],
            rev: 0,
            res: core::ptr::null_mut(),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct LimineKernelFileResponse {
    pub rev: u64,
    pub file: *mut limine::file::File,
}
