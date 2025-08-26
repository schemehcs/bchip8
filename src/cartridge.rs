use crate::opcode;
use std::fs;
use std::path;

pub const CARTRIDGE_ADDRESS: usize = 0x200;

pub struct Assemable {
    opcode: u16,
    operation: opcode::Operation,
}

impl Assemable {
    fn new(opcode: u16, operation: opcode::Operation) -> Self {
        Assemable { opcode, operation }
    }
}

pub fn load_cartridge(path: &path::PathBuf) -> anyhow::Result<Vec<u8>> {
    Ok(fs::read(path)?)
}

pub fn disassemble_cartridge(cartridge: &[u8]) -> Vec<Assemable> {
    cartridge
        .chunks(2)
        .map(|c| {
            let opcode = if c.len() == 1 {
                u16::from_be_bytes([c[0], 0])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            };
            Assemable::new(opcode, opcode::parse_opcode(opcode))
        })
        .collect()
}

pub fn debug_cartridge(cartridge: &[u8]) {
    let mut addr = CARTRIDGE_ADDRESS;
    for d in disassemble_cartridge(cartridge) {
        println!("{:0>12x}: [{:0>4x}] {}", addr, d.opcode, d.operation);
        addr += 2;
    }
}
