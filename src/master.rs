use crate::{controls, dma, hardware, interrupts, timer};
use std::io::{stdin, stdout, Read, Write};

const H_BLANK: u8 = 0;
const V_BLANK: u8 = 1;
const PX_TRANSFER: u8 = 2;

pub struct Master {
    pub tick: u64,
    pub mode: u8,
    pub previous_mode: u8,
    pub step_by_step: bool,
    pub line_by_line: bool,
    pub screen_by_screen: bool,
    pub log: bool,
}

impl Master {
    pub fn step(
        &mut self,
        cpu: &mut hardware::Cpu,
        gpu: &mut hardware::Gpu,
        timer: &mut timer::Timer,
        controls: &mut controls::Controls,
        ram: &mut [u8; 0x10000],
    ) {
        //Check for interrupts, if none juste add 1 to PC
        //if cpu.get_pc() == 0x2000 { self.step_by_step = true;self.log=true;}
        interrupts::interrupt_check(cpu, ram);
        let instruct: &hardware::Instruct = cpu.fetch(ram[cpu.get_pc() as usize]);
        let argc: u8 = instruct.argc;

        if instruct.name.contains("/!\\")
            && (ram[cpu.get_pc() as usize] != 0xcb && ram[cpu.get_pc() as usize + 1] != 0x37)
            || self.step_by_step
        {
            self.log = true;
            self.maxi_debug_print(&cpu, &timer, &ram, &controls, &instruct);
            wait();
        }
        //println!("Pc: {:#06x}", cpu.get_pc());

        self.tick = self.tick.wrapping_add(instruct.ticks as u64);

        timer.update(instruct.ticks, ram);

        controls.update_ram(ram);

        let opcode = instruct.opcode;
        cpu.exec(opcode, ram);

        //adding temporary ticks from the cpu
        self.tick = self.tick.wrapping_add(cpu.get_ticks() as u64);

        let mut delay = false;
        //Ie delay
        if ram[cpu.get_pc() as usize] == 0xFB {
            delay = true;
        }

        cpu.set_pc(cpu.get_pc().wrapping_add((argc as u16) + 1));

        dma::update_dma(ram);

        if delay {
            self.step(cpu, gpu, timer, controls, ram);
            cpu.mie = true;
        }
    }

    pub fn screen(
        &mut self,
        cpu: &mut hardware::Cpu,
        gpu: &mut hardware::Gpu,
        timer: &mut timer::Timer,
        controls: &mut controls::Controls,
        ram: &mut [u8; 0x10000],
    ) {
        ram[0xFF44] = 0;
        for i in 0..144 {
            while self.tick < 114 {
                if self.tick > 63 {
                    self.mode = H_BLANK;
                } else {
                    self.mode = PX_TRANSFER;
                }
                //print!("{esc}c", esc = 27 as char);
                //println!("SCREEN STATE__________________________________");
                //println!("State: Printing");
                //println!("Line: {}",i);
                //println!("Mode: {}",self.mode);
                //println!(" ");
                self.step(cpu, gpu, timer, controls, ram);
                self.lcd_stat(i, ram);
                if self.step_by_step {
                    wait();
                }
            }
            self.tick = 0;
            gpu.push_line(ram);

            if self.line_by_line {
                wait();
            }

            ram[0xff44] += 1;
        }

        ram[0xFF0F] = ram[0xFF0F] | 0b1;
        self.mode = V_BLANK;

        for _j in 0..10 {
            while self.tick < 114 {
                //print!("{esc}c", esc = 27 as char);
                //println!("SCREEN STATE__________________________________");
                //println!("State: V-Blank");
                //println!("Mode: {}",self.mode);
                //println!(" ");
                self.step(cpu, gpu, timer, controls, ram);
                self.lcd_stat(254, ram);
                if self.step_by_step {
                    wait();
                }
            }
            self.tick = 0;
            if self.line_by_line {
                wait();
            }
            ram[0xff44] += 1;
        }

        if self.screen_by_screen {
            wait();
        }
    }

    pub fn maxi_debug_print(
        &self,
        cpu: &hardware::Cpu,
        timer: &timer::Timer,
        ram: &[u8; 0x10000],
        controls: &controls::Controls,
        instruc: &hardware::Instruct,
    ) {
        if self.log {
            println!("Pc: {:#06x}", cpu.get_pc());
            println!("OPERATION____________________________________");
            println!("Count:{}", self.tick);
            println!("Pc: {:#06x}", cpu.get_pc());
            println!(
                "Ram values: {:#04x} {:#04x} {:#04x}",
                ram[cpu.get_pc() as usize],
                ram[(cpu.get_pc() + 1) as usize],
                ram[(cpu.get_pc() + 2) as usize]
            );
            println!("Name:{}", &instruc.name);
            println!("Instruction: {}", &instruc.desc);
            println!("Ticks: {}", &instruc.ticks);
            println!();
            println!("CPU STATE____________________________________");
            println!("a:{} / {:#04x}", cpu.get_a(), cpu.get_a());
            println!("f:{} / {:#04x}", cpu.get_f(), cpu.get_f());
            println!("b:{} / {:#04x}", cpu.get_b(), cpu.get_b());
            println!("c:{} / {:#04x}", cpu.get_c(), cpu.get_c());
            println!("d:{} / {:#04x}", cpu.get_d(), cpu.get_d());
            println!("e:{} / {:#04x}", cpu.get_e(), cpu.get_e());
            println!("h:{} / {:#04x}", cpu.get_h(), cpu.get_h());
            println!("l:{} / {:#04x}", cpu.get_l(), cpu.get_l());
            println!("sp:{:#04x}", cpu.get_sp());
            println!("mie: {}", cpu.get_mie());
            println!("0xFFFF: {:#010b}", ram[0xFFFF]);
            println!("0xFF0F: {:#010b}", ram[0xFF0F]);
            println!();
            println!("FLAGS STATE__________________________________");
            let flags = cpu.get_flags();
            println!("Z:{}", flags.z);
            println!("N:{}", flags.n);
            println!("H:{}", flags.h);
            println!("C:{}", flags.c);
            println!();
            println!("TIMER STATE__________________________________");
            println!("Divider:{:#04x}", ram[0xff04]);
            println!("Divider ticks:{}", timer.divider_ticks);
            println!("Timer enable:{}", timer.timer_enb);
            println!("Timer division:{}", timer.division);
            println!("Timer:{:#04x}", ram[0xff05]);
            println!("Timer ticks:{}", timer.timer_ticks);
            println!();
            println!("INPUT STATE__________________________________");
            println!(
                "Buttons: U:{} D:{} L:{} R:{} A:{} B:{} SE:{} ST:{}",
                controls.up,
                controls.down,
                controls.left,
                controls.right,
                controls.a,
                controls.b,
                controls.select,
                controls.start
            );
            println!("0XFF00: {:#010b}", ram[0xFF00]);
            println!();
            println!("WARNING______________________________________");
        }
    }

    pub fn lcd_stat(&mut self, line: u8, ram: &mut [u8; 0x10000]) {
        if ram[0xFF41] & 0b0100000 > 0 && line == ram[0xFF45] && self.previous_mode == H_BLANK {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: LY=LYC");}
            self.previous_mode = self.mode;
        }
        if ram[0xFF41] & 0b00001000 > 0 && self.mode == H_BLANK && self.mode != self.previous_mode {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: H_BLANK");}
            self.previous_mode = self.mode;
        }
        if ram[0xFF41] & 0b00010000 > 0 && self.mode == V_BLANK && self.mode != self.previous_mode {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: V_BLANK");}
            self.previous_mode = self.mode;
        }
        if ram[0xFF41] & 0b0010000 > 0
            && self.mode == PX_TRANSFER
            && self.mode != self.previous_mode
        {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: PX_TRANSFER");}
            self.previous_mode = self.mode;
        }
    }
}

pub fn wait() {
    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
    print!("{esc}c", esc = 27 as char);
}