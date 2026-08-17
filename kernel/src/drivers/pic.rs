pub unsafe fn disable_pic() {
    use x86_64::instructions::port::Port;
    unsafe {
        let mut p1_cmd: Port<u8> = Port::new(0x20);
        let mut p1_data: Port<u8> = Port::new(0x21);
        let mut p2_cmd: Port<u8> = Port::new(0xA0);
        let mut p2_data: Port<u8> = Port::new(0xA1);

        // ICW1
        p1_cmd.write(0x11);
        p2_cmd.write(0x11);
        // ICW2
        p1_data.write(0x90);
        p2_data.write(0x98);
        // ICW3
        p1_data.write(0x04);
        p2_data.write(0x02);
        // ICW4
        p1_data.write(0x01);
        p2_data.write(0x01);
        p1_data.write(0xFF);
        p2_data.write(0xFF);
    }
    crate::println_serial!("PIC 8259 disabled");
}
