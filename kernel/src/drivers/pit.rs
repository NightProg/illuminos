use x86_64::instructions::port::Port;
const PIT_CH0: u16 = 0x40;
const PIT_CH2: u16 = 0x42;
const PIT_CMD: u16 = 0x43;
const PIT_CTL: u16 = 0x61;
const PIT_FREQ: u32 = 1193182;

fn init_pit() {
    let frequency: u32 = 100; // 100 Hz
    let divisor = PIT_FREQ / frequency;
    unsafe {
        let mut cmd = Port::new(PIT_CMD);
        cmd.write(0x34u8);
        let mut ch0 = Port::new(PIT_CH0);
        ch0.write((divisor & 0xFF) as u8); // low byte
        ch0.write((divisor >> 8) as u8); // high byte
    }
}

pub unsafe fn pit_wait_ms(ms: u32) {
    unsafe {
        let ticks = PIT_FREQ / 1000 * ms;

        let mut cmd: Port<u8> = Port::new(PIT_CMD);
        let mut ch2: Port<u8> = Port::new(PIT_CH2);
        let mut ctl: Port<u8> = Port::new(PIT_CTL);

        let old_ctl = ctl.read();
        ctl.write((old_ctl & 0xFD) | 0x01);

        cmd.write(0b10110100);

        ch2.write((ticks & 0xFF) as u8);
        ch2.write((ticks >> 8) as u8);
        let s = ctl.read();
        ctl.write(s | 0x03);

        loop {
            if ctl.read() & 0x20 != 0 {
                break;
            }
        }
    }
}
