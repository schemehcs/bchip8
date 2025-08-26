mod cartridge;
mod console;
mod font;
mod machine;
mod opcode;

use clap::Parser;
use machine::Machine;
use std::path;
use std::{fs, time::Duration};

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Cli {
    #[arg(value_name = "cartridge")]
    cartridge: path::PathBuf,

    #[arg(long, short, default_value_t = false)]
    disassemble: bool,

    #[arg(long, short, default_value_t = 1000)]
    cycle_micro: u64,

    #[arg(long, short, default_value = "chip8.log")]
    log_file: path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let log_file = fs::File::create(cli.log_file).expect("Failed to create log file");
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    let cartridge = cartridge::load_cartridge(&cli.cartridge)?;
    if cli.disassemble {
        cartridge::debug_cartridge(&cartridge);
        return Ok(());
    }

    let rng = rand::rng();
    let mut machine = Machine::new(rng, Duration::from_micros(cli.cycle_micro))?;
    machine.load_font(font::FONT_ADDRESS, font::load_default_font())?;
    machine.load_cartridge(cartridge::CARTRIDGE_ADDRESS, &cartridge)?;
    machine.boot()?;

    Ok(())
}
