use mynes::bus::Bus;
use mynes::cartridge::Rom;
use mynes::cpu::state::CPU;
use mynes::trace::*;

fn main() {
    //load the nestest
    let bytes: Vec<u8> = std::fs::read("nestest.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();

    let bus = Bus::new(rom);
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.program_counter = 0xC000;
    cpu.run_with_callback(move |cpu| {
        println!("{}", trace(cpu));
    });
}
