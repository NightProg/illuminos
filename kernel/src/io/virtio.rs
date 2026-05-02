use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Page, PageTableFlags, PhysFrame};
use crate::allocator::paging::PagingManager;
use crate::io::pci::{pci_read, PciDevice};
use crate::println;

pub unsafe fn scan_notify_cap(pci: &PciDevice) -> Option<(u8, u32)> {
    let mut offset: u16 = 0x100;

    loop {
        let cap_header = pci.read_config_mmio(offset).unwrap();
        let cap_type = (cap_header & 0xFF) as u8;

        // VIRTIO_PCI_CAP_NOTIFY_CFG
        if cap_type == 0x10 {
            // offset + 2 → bar
            let bar = pci.read_config_mmio(offset + 2).unwrap();
            // offset + 4 → offset dans le BAR
            let offset_in_bar = pci.read_config_mmio(offset + 4).unwrap();

            println!(
                "Found notify capability: BAR = {}, Offset = {:#x}",
                bar, offset_in_bar
            );

            return Some((bar as u8, offset_in_bar));
        }

        // Lire la taille ou le pointeur vers la prochaine capacité
        let cap_len = pci.read_config_mmio(offset + 1).unwrap();
        if cap_len < 8 {
            break; // Capacité invalide ou fin
        }

        offset = offset.wrapping_add(cap_len as u16);
        if offset == 0 || offset >= 0x100 + 0x100 {
            break;
        }
    }

    None
}



static mut DESCS: [VirtqDesc; 8] = [VirtqDesc::default(); 8];
static mut AVAIL: VirtqAvail = VirtqAvail::default();
static mut USED: VirtqUsed = VirtqUsed::default();

#[repr(C)]
#[derive(Debug)]
pub struct VirtioPciCommonCfg {
    pub device_feature_select: u32,
    pub device_feature: u32,
    pub driver_feature_select: u32,
    pub driver_feature: u32,
    pub msix_config: u16,
    pub num_queues: u16,
    pub device_status: u8,
    pub config_generation: u8,
    pub queue_select: u16,
    pub queue_size: u16,
    pub queue_msix_vector: u16,
    pub queue_enable: u16,
    pub queue_notify_off: u16,
    pub queue_desc: u64,
    pub queue_driver: u64,
    pub queue_device: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

impl VirtqDesc {
    pub const fn default() -> Self {
        Self {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }
    }
}

#[repr(C)]
pub struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; 8], // Taille de file arbitraire
}

impl VirtqAvail {
    pub const fn default() -> Self {
        Self {
            flags: 0,
            idx: 0,
            ring: [0; 8],
        }
    }
}

#[repr(C)]
#[derive(Copy)]
#[derive(Clone)]
pub struct VirtqUsedElem {
    id: u32,
    len: u32,
}

impl VirtqUsedElem {
    pub const fn default() -> Self {
        Self {
            id: 0,
            len: 0,
        }
    }
}

#[repr(C)]
pub struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; 8],
}

impl VirtqUsed {
    pub const fn default() -> Self {
        Self {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElem::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct VirtioGpuCtrlHdr {
    type_: u32,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtioGpuRespDisplayInfo {
    hdr: VirtioGpuCtrlHdr,
    pmodes: [VirtioGpuDisplayOne; 16],
}

#[repr(C)]
#[derive(Debug)]
struct VirtioGpuDisplayOne {
    r: VirtioRect,
    enabled: u32,
    flags: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtioRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

static mut REQUEST: VirtioGpuCtrlHdr = VirtioGpuCtrlHdr {
    type_: 0x0100, // VIRTIO_GPU_CMD_GET_DISPLAY_INFO
    flags: 0,
    fence_id: 0,
    ctx_id: 0,
    padding: 0,
};

static mut RESPONSE: VirtioGpuRespDisplayInfo = unsafe { core::mem::zeroed() };


#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioPciCap {
    cap_vndr: u8,   // 0x09
    cap_next: u8,
    cap_len: u8,
    cfg_type: u8,
    bar: u8,
    id: u8,
    padding: [u8; 2],
    offset: u32,
    length: u32,
}

impl PciDevice {
    pub unsafe fn find_common_cfg(&self) -> Option<(u8 /*bar*/, u32 /*offset*/, u32 /*length*/)> {
        // Lire le registre status (0x06) pour vérifier si capabilities sont présentes
        let status = (pci_read(self.bus, self.device, self.function, 0x04) >> 16) as u16;
        if status & (1 << 4) == 0 {
            return None;
        }

        // Lire pointeur de capability list (0x34)
        let mut cap_ptr = (pci_read(self.bus, self.device, self.function, 0x34) & 0xFF) as u8;
        while cap_ptr != 0 {
            let cap_header = pci_read(self.bus, self.device, self.function, cap_ptr);
            let cap_id = (cap_header & 0xFF) as u8;
            let cap_next = ((cap_header >> 8) & 0xFF) as u8;

            if cap_id == 0x09 {
                // Lire toute la structure VirtioPciCap (16 octets)
                let mut cap_data = [0u8; core::mem::size_of::<VirtioPciCap>()];
                for i in 0..(cap_data.len() / 4) {
                    let word = pci_read(self.bus, self.device, self.function, cap_ptr + (i * 4) as u8);
                    cap_data[i * 4..(i + 1) * 4].copy_from_slice(&word.to_le_bytes());
                }

                let cap: VirtioPciCap = core::ptr::read(cap_data.as_ptr() as *const _);

                if cap.cfg_type == 1 {
                    return Some((cap.bar, cap.offset, cap.length));
                }
            }

            cap_ptr = cap_next;
        }

        None
    }
}

#[derive(Debug)]
pub struct VirtioPciDevice {
    pub pci: PciDevice,
    pub bar_index: u8,
    pub common_cfg: *mut VirtioPciCommonCfg,
    pub is_modern: bool,
}

impl VirtioPciDevice {

    pub unsafe fn new(pci: PciDevice, paging_manager: &mut PagingManager) -> Option<Self> {
        if !pci.is_virtio() {
            return None;
        }

        let (bar, offset, length) = pci.find_common_cfg()?;
        let bar_addr = pci.get_bar(bar)?;
        let common_cfg_addr = (bar_addr + offset as u64) as *mut u8;
        println!("common_cfg @ BAR{} + {:#x} (length: {})", bar, offset, length);



        let common_cfg = bar_addr as *mut u8;
        let v = VirtAddr::new(0x30_000000);
        let page = Page::containing_address(v);
        let phys_addr = PhysAddr::new(bar_addr);
        let phys_frame = PhysFrame::containing_address(phys_addr);
        paging_manager.map_page(page, phys_frame, PageTableFlags::WRITABLE | PageTableFlags::PRESENT);


        Some(Self {
            pci,
            bar_index: bar,
            common_cfg: 0x30_000000 as *mut VirtioPciCommonCfg,
            is_modern: false,
        })
    }

    pub unsafe fn read_reg32(&self, offset: usize) -> u32 {
        core::ptr::read_volatile(self.common_cfg.add(offset) as *const u32)
    }

    pub unsafe fn write_reg32(&self, offset: usize, value: u32) {
        core::ptr::write_volatile(self.common_cfg.add(offset) as *mut u32, value);
    }

    pub fn virtio_device_id(&self) -> u32 {
        unsafe { self.read_reg32(0x00) } // Legacy: offset 0
    }

    pub fn negotiate_features(&self, features: u32) {
        unsafe {
            self.write_reg32(0x10, features);
        }
    }

    pub fn gpu_init(&mut self, paging_manager: &mut PagingManager) {
        unsafe {
            (*self.common_cfg).queue_select = 0;
            (*self.common_cfg).queue_size = 8;

            (*self.common_cfg).queue_desc = &DESCS as *const _ as u64;
            (*self.common_cfg).queue_driver = &AVAIL as *const _ as u64;
            (*self.common_cfg).queue_device = &USED as *const _ as u64;

            (*self.common_cfg).queue_enable = 1;

            unsafe {
                DESCS[0] = VirtqDesc {
                    addr: &REQUEST as *const _ as u64,
                    len: core::mem::size_of::<VirtioGpuCtrlHdr>() as u32,
                    flags: 0, // Device reads this
                    next: 1,
                };

                DESCS[1] = VirtqDesc {
                    addr: &RESPONSE as *const _ as u64,
                    len: core::mem::size_of::<VirtioGpuRespDisplayInfo>() as u32,
                    flags: 1, // Device writes this
                    next: 0,
                };

                AVAIL.ring[0] = 0;
                AVAIL.idx = 1;

                let (notify_bar, notify_offset) = scan_notify_cap(&self.pci).unwrap();

                let queue_notify_off = (*self.common_cfg).queue_notify_off as u16;
                let notify_base = notify_bar as u64; // Adresse de base pour le BAR de notification
                let notify_addr = notify_base + (notify_offset as u64) + (queue_notify_off as u64 * 2);
                let notify_frame = PhysFrame::containing_address(PhysAddr::new(notify_addr));
                let page = Page::containing_address(VirtAddr::new(notify_addr));
                paging_manager.map_page(page, notify_frame, PageTableFlags::WRITABLE | PageTableFlags::PRESENT);

                // Vérifier si l'adresse de notification est correctement mappée dans l'espace mémoire
                let notify_addr_phys = notify_addr as *mut u16;
                unsafe {
                    // Accès direct à la mémoire physique, l'adresse doit être mappée correctement
                    // Assurer l'intégrité de l'adresse avant d'écrire
                    *(notify_addr_phys) = 0;  // Notifier le périphérique avec la valeur appropriée
                }

            }

            while USED.idx == 0 {
                // spin
            }

        }
    }

}

