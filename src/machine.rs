use std::thread;
use std::time::Duration;
use std::time::Instant;

use anyhow;
use log::{Level, log_enabled, info, warn, trace};
use rand::Rng;
use crate::console;
use crate::console::Console;
use crate::console::Key;
use crate::console::KeyEvent;
use crate::opcode;

pub const MEMORY_SIZE: usize = 0x1000;
pub const REGISTER_COUNT: usize = 0x10;
pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const TICK_RATE: Duration = Duration::from_millis(16);

const SPRITE_MASK: [u8; 8] = [
    1 << 7,
    1 << 6,
    1 << 5,
    1 << 4,
    1 << 3,
    1 << 2,
    1 << 1,
    1 << 0,
];

enum GetKeyState {
    None,
    Paused,
    Pressed(u8),
    Released(u8),
}

pub struct Machine<R> 
where R: Rng{
    running: bool,
    cycle: Duration,
    display_buffer: [[bool; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    console: Console,
    cartridge_address: usize,
    font_address: usize,
    display_buffer_dirty: bool,
    key_state: [bool; 16],
    get_key_state: GetKeyState,
    rng: R,
    tick_cnt: u128,
    tick_at: Instant,
    register_i: u16,
    register_pool: [u8; REGISTER_COUNT],
    delay_timer: u8,
    sound_timer: u8,
    memory: [u8; MEMORY_SIZE],
    stack: Vec<usize>,
    pc: usize,
}

impl<R> Machine<R>
where R: Rng {
    pub fn new(rng: R, cycle: Duration) -> anyhow::Result<Self> {
        let console = console::init()?;
        Ok(Machine {
            running: false,
            cycle: cycle,
            display_buffer: [[false; DISPLAY_WIDTH as usize]; DISPLAY_HEIGHT as usize],
            console: console,
            cartridge_address: 0x0,
            font_address: 0x0,
            display_buffer_dirty: false,
            key_state: [false; 16],
            get_key_state: GetKeyState::None,
            rng: rng,
            tick_cnt: 0,
            tick_at: Instant::now(),
            register_i: 0x0,
            register_pool: [0u8; REGISTER_COUNT],
            delay_timer: 0x0,
            sound_timer: 0x0,
            memory: [0; MEMORY_SIZE],
            stack: Vec::new(),
            pc: 0x0,
        })
    }

    pub fn boot(&mut self) -> anyhow::Result<()> {
        self.reset_tick();
        self.pc = self.cartridge_address as usize;
        self.running = true;
        let mut cycle_at = Instant::now();
        while self.running {
            match self.get_key_state {
                GetKeyState::None | GetKeyState::Released(_) => {
                    let last_cycle_elapsed = cycle_at.elapsed();
                    if last_cycle_elapsed <= self.cycle {
                        thread::sleep(self.cycle - last_cycle_elapsed);
                    }
                    cycle_at = Instant::now();
                    self.step()?;
                },
                _ => {},
            };

            self.handle_key_events()?;
            self.display()?;

            if self.tick_at.elapsed() >= TICK_RATE {
                self.tick()?;
            }
        }

        self.on_halt();
        Ok(())
    }

    fn on_halt(&mut self) {
        self.console.restore();
    }

    pub fn trace_machine(&self) -> anyhow::Result<()> {
        if !log_enabled!(Level::Trace) {
            return Ok(());
        }
        let mut buf = String::new();
        buf.push_str("[- Machine -]\n");
        buf.push_str("  <r> ");
        for i in 0..REGISTER_COUNT {
            buf.push_str(&format!("[{:x}|{:0>2x}] ", i, self.get_register(i as u8)?));
        }
        buf.push_str("\n");
        buf.push_str(&format!("  <i> {:0>2x}\n", self.get_register_i()));
        buf.push_str("  <k> ");
        for k in 0..=0xF {
            let stat: u8 = if self.key_state[k] {1} else {0};
            buf.push_str(&format!("[{:x}|{}] ", k, stat));
        }
        buf.push_str("\n");
        trace!("{}", buf);
        Ok(())
    }

    pub fn trace_display(&self) {
        if !log_enabled!(Level::Trace) {
            return;
        }
        let mut buf = String::new();
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                if self.display_buffer[y as usize][x as usize] {
                    buf.push_str("██");
                } else {
                    buf.push_str("  ");
                }
            }
            buf.push_str("\n");
        }
        trace!("{}", buf);
    }

    pub fn load(&mut self, address: usize, data: &[u8]) -> anyhow::Result<()> {
        if address + data.len() >= MEMORY_SIZE {
            anyhow::bail!("memory overflow");
        }
        let sl = &mut self.memory[address..address + data.len()];
        sl.copy_from_slice(data);
        Ok(())
    }

    pub fn load_font(&mut self, address: usize, font: &[u8]) -> anyhow::Result<()> {
        self.load(address, font)?;
        self.font_address = address;
        Ok(())
    }

    pub fn load_cartridge(&mut self, address: usize, cart: &[u8]) -> anyhow::Result<()> {
        self.load(address, cart)?;
        self.cartridge_address = address;
        Ok(())
    }

    fn clear_display(&mut self) {
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                self.display_buffer[y][x] = false;
            }
        }
    }

    fn update_delay_timer(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
    }

    fn update_sound_timer(&mut self) -> bool {
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            if self.sound_timer == 0 {
                return true;
            } else {
                return false;
            }
        }
        return false;
    }

    fn draw(&mut self, x: usize, y: usize, height: u8) -> anyhow::Result<()> {
        let x = x % DISPLAY_WIDTH;
        let y = y % DISPLAY_HEIGHT;
        let mut i_addr = self.get_register_i();
        self.clr_vf();
        for yi in 0..height as usize {
            let srow_map = self.get_memory(i_addr as usize)?;
            let ye = y + yi;
            if ye >= DISPLAY_HEIGHT {
                break;
            }
            for xi in 0..8 {
                let xe = x + xi;
                if xe >= DISPLAY_WIDTH {
                    break;
                }
                let cur_dis = self.display_buffer[ye][xe];
                let sprite_dis = (srow_map & SPRITE_MASK[xi as usize]) != 0;
                let new_dis = cur_dis ^ sprite_dis;
                if cur_dis != new_dis {
                    self.display_buffer[ye][xe] = new_dis;
                    self.display_buffer_dirty = true;
                    if cur_dis {
                        self.set_vf();
                    }
                }
            }
            i_addr += 1;
        }
        Ok(())
    }

    fn reset_tick(&mut self) {
        self.tick_cnt = 0;
        self.tick_at = Instant::now();
    }

    fn tick(&mut self) -> anyhow::Result<()> {
        self.tick_at = Instant::now();
        self.tick_cnt += 1;
        self.on_tick()?;
        Ok(())
    }

    fn on_tick(&mut self) -> anyhow::Result<()> {
        self.update_delay_timer();
        self.update_sound_timer();
        Ok(())
    }

    fn handle_key_events(&mut self) -> anyhow::Result<()> {
        let key_events = self.console.get_key_events(self.next_tick_left())?;
        if !key_events.is_empty() {
            info!("(KeyEvents): {:?}", key_events);
        }
        for ke in key_events {
            match ke {
                KeyEvent::Pressed(k) => {
                    match k {
                        Key::Quit => self.running = false,
                        Key::Num(n) => {
                            if matches!(self.get_key_state, GetKeyState::Paused) {
                                self.get_key_state = GetKeyState::Pressed(n);
                            }
                            self.key_state[n as usize] = true;
                        }
                    }
                },
                KeyEvent::Released(k) => {
                    match k {
                        Key::Num(n) => {
                            if let GetKeyState::Pressed(k) = self.get_key_state {
                                if k == n {
                                    self.get_key_state = GetKeyState::Released(k);
                                }
                            }
                            self.key_state[n as usize] = false;
                        },
                        _ => {}    
                    }
                },
            }
        }
        Ok(())
    }

    fn display(&mut self) -> anyhow::Result<()> {
        if self.display_buffer_dirty {
            self.trace_display();
            self.console.draw(&self.display_buffer)?;
            self.display_buffer_dirty = false;
        }
        Ok(())
    }

    fn advance_pc(&mut self, op_distance: usize) -> anyhow::Result<()> {
        let distance = op_distance * 2;
        if self.pc + distance >= MEMORY_SIZE {
            anyhow::bail!("pc overflow");
        }
        self.pc += distance;
        Ok(())
    }

    fn advance(&mut self) -> anyhow::Result<()> {
        self.advance_pc(1)?;
        Ok(())
    }

    fn set_pc(&mut self, pc: usize) -> anyhow::Result<()> {
        if pc >= MEMORY_SIZE {
            anyhow::bail!("pc overflow");
        }
        self.pc = pc;
        Ok(())
    }

    fn set_register(&mut self, reg_id: u8, val: u8) -> anyhow::Result<()> {
        let reg_id: usize = reg_id as usize;
        if reg_id >= REGISTER_COUNT {
            anyhow::bail!("invalid general register id {}", reg_id);
        }
        self.register_pool[reg_id] = val;
        Ok(())
    }

    fn set_vf(&mut self) {
        self.set_register(0xF, 1).unwrap();
    }

    fn clr_vf(&mut self) {
        self.set_register(0xF, 0).unwrap();
    }

    fn get_register(&self, reg_id: u8) -> anyhow::Result<u8> {
        let reg_id: usize = reg_id as usize;
        if reg_id >= REGISTER_COUNT {
            anyhow::bail!("invalid general register id {}", reg_id);
        }
        Ok(self.register_pool[reg_id])
    }

    fn get_register_i(&self) -> u16 {
        self.register_i
    }

    fn set_register_i(&mut self, val: u16) -> anyhow::Result<()> {
        if val >= (MEMORY_SIZE as u16) {
            anyhow::bail!("reg i over flow {:0>4x}", val);
        }
        self.register_i = val;
        Ok(())
    }

    fn get_memory(&self, addr: usize) -> anyhow::Result<u8> {
        if addr >= MEMORY_SIZE {
            anyhow::bail!("memory overflow");
        }
        Ok(self.memory[addr])
    }

    fn set_memory(&mut self, addr: usize, data: u8) -> anyhow::Result<()> {
        if addr >= MEMORY_SIZE {
            anyhow::bail!("memory overflow");
        }
        self.memory[addr] = data;
        Ok(())
    }

    fn get_opcode(&self, addr: usize) -> anyhow::Result<u16> {
        Ok(u16::from_be_bytes([self.get_memory(addr)?, self.get_memory(addr + 1)?]))
    }

    fn get_operation(&mut self) -> anyhow::Result<opcode::Operation> {
        Ok(opcode::parse_opcode(self.get_opcode(self.pc)?))
    }

    fn next_tick_left(&self) -> Duration {
        let elapsed = self.tick_at.elapsed();
        if TICK_RATE >= elapsed {
            return TICK_RATE - elapsed;
        } else {
            return Duration::from_millis(0);
        }
    }

    fn step(&mut self) -> anyhow::Result<()> {
        let operation = self.get_operation()?;
        trace!("[{:x}] {:x}: {}", self.pc, self.get_opcode(self.pc)?, &operation);
        use opcode::Operation::*;
        match operation.clone() {
            CallSysC(_) => {
                self.advance()?;
            },
            Clear => {
                self.clear_display();
                self.advance()?;
            },
            Return => {
                match self.stack.pop() {
                    Some(ret_pc) => self.set_pc(ret_pc as usize)?,
                    None => anyhow::bail!("call stack empty, no where to return"),
                }
            },
            JumpC(c) => {
                self.set_pc(c as usize)?;
            },
            CallC(c) => {
                self.advance()?;
                self.stack.push(self.pc);
                self.set_pc(c as usize)?;
            },
            SkipEqC(x, c) => {
                if c == self.get_register(x)? {
                    self.advance_pc(2)?;
                } else {
                    self.advance()?;
                }
            },
            SkipNeC(x, c) => {
                if c != self.get_register(x)? {
                    self.advance_pc(2)?;
                } else {
                    self.advance()?;
                }
            },
            SkipEq(x, y) => {
                if self.get_register(x)? == self.get_register(y)? {
                    self.advance_pc(2)?;
                } else {
                    self.advance()?;
                }
            },
            SetC(x, c) => {
                self.set_register(x, c)?;
                self.advance()?;
            },
            AddC(x, c) => {
                self.set_register(x, self.get_register(x)?.wrapping_add(c))?;
                self.advance()?;
            },
            Set(x, y) => {
                self.set_register(x, self.get_register(y)?)?;
                self.advance()?;
            },
            Or(x, y) => {
                self.set_register(x,
                    self.get_register(x)? | self.get_register(y)?
                )?;
                self.clr_vf();
                self.advance()?;
            },
            And(x, y) => {
                self.set_register(x,
                    self.get_register(x)? & self.get_register(y)?
                )?;
                self.clr_vf();
                self.advance()?;
            },
            Xor(x, y) => {
                self.set_register(x,
                    self.get_register(x)? ^ self.get_register(y)?
                )?;
                self.clr_vf();
                self.advance()?;
            },                
            Add(x, y) => {
                let xv = self.get_register(x)?;
                let yv = self.get_register(y)?;
                let res = xv.wrapping_add(yv);
                self.set_register(x, res)?;
                if res < xv {
                    self.set_vf();
                } else {
                    self.clr_vf();
                }

                self.advance()?;
            },
            Sub(x, y) => {
                let xv = self.get_register(x)?;
                let yv = self.get_register(y)?;
                let res = xv.wrapping_sub(yv);
                self.set_register(x, res)?;
                if res > xv { 
                    self.clr_vf(); // underflowed
                } else {
                    self.set_vf();
                }
                self.advance()?;
            },
            Shr(x, y) => {
                let yv = self.get_register(y)?;
                self.set_register(x, yv.unbounded_shr(1))?;
                if yv & 1 == 1 {
                    self.set_vf();
                } else {
                    self.clr_vf();
                }
                self.advance()?;
            },
            SubRev(x, y) =>  {
                let xv = self.get_register(x)?;
                let yv = self.get_register(y)?;
                let res = yv.wrapping_sub(xv);
                self.set_register(x, res)?;
                if res > yv { 
                    self.clr_vf(); // underflowed
                } else {
                    self.set_vf();
                }
                self.advance()?;
            },
            Shl(x, y) => {
                let yv = self.get_register(y)?;
                self.set_register(x, yv.unbounded_shl(1))?;
                if yv & 0x80 == 0x80 {
                    self.set_vf();
                } else {
                    self.clr_vf();
                }
                self.advance()?;
            },
            SkipNe(x, y) => {
                if self.get_register(x)? != self.get_register(y)? {
                    self.advance_pc(2)?;
                } else {
                    self.advance()?;
                }
            },
            SetIC(c) => {
                self.set_register_i(c)?;
                self.advance()?;
            },
            JumpV0C(c) => {
                let entry = self.get_register(0)? as u16;
                let _ = match entry.checked_add(c) {
                    Some(addr) => self.set_pc(addr as usize)?,
                    None => anyhow::bail!("jumpV0C address overflow"),
                };
            },
            RandC(x, c) => {
                let randv: u8 = self.rng.random();
                self.set_register(x, randv & c)?;
                self.advance()?;
            },
            DrawC(x, y, c) => {
                let x = self.get_register(x)?;
                let y = self.get_register(y)?;
                self.draw(x as usize, y as usize, c)?;
                self.advance()?;
            },
            SkipEqKey(x) => {
                let key = self.get_register(x)?;
                let kstat = self.key_state[key as usize];
                info!("(SkipEqKey)[k|{:x}] [stat|{}]", key, kstat);
                if kstat {
                    self.advance_pc(2)?;
                } else {
                    self.advance()?;
                }
            },
            SkipNeKey(x) => {
                let key = self.get_register(x)?;
                let kstat = self.key_state[key as usize];
                info!("(SkipNeKey)[k|{:x}] [stat|{}]", key, kstat);
                if !kstat {
                    self.advance_pc(2)?;
                } else {
                    self.advance()?
                }
            },
            GetDelayTimer(x) => {
                self.set_register(x, self.delay_timer)?;
                self.advance()?;
            },
            GetKey(x) => {
                match self.get_key_state {
                    GetKeyState::None => {
                        info!("(GetKey::None) machine -> paused");
                        self.get_key_state = GetKeyState::Paused;
                    },
                    GetKeyState::Released(key) => {
                        info!("(GetKey::Released({:0>2x})), machine -> none", key);
                        self.get_key_state = GetKeyState::None;
                        self.set_register(x, key)?;
                        self.advance()?
                    },
                    _ => {}
                }
            },
            SetDelayTimer(x) => {
                self.delay_timer = self.get_register(x)?;
                self.advance()?;
            },
            SetSoundTimer(x) => {
                self.sound_timer = self.get_register(x)?;
                self.advance()?;
            },
            AddI(x) => {
                self.set_register_i(
                    self.get_register_i() + self.get_register(x)? as u16
                )?;
                self.advance()?;
            },
            SetIFont(x) => {
                let xv = self.get_register(x)?;
                let offset: usize = (xv as usize).checked_mul(self.font_address).unwrap();
                self.set_register_i(self.font_address.checked_add(offset).unwrap() as u16)?;
                self.advance()?;
            },
            Bcd(x) => {
                let xv = self.get_register(x)?;
                let n0 = xv % 10;
                let n1 = (xv / 10) % 10;
                let n2 = (xv / 100) % 10;
                let address = self.get_register_i() as usize;
                self.set_memory(address, n2)?;
                self.set_memory(address + 1, n1)?;
                self.set_memory(address + 2, n0)?;
                self.advance()?;
            },
            Store(x) => {
                let mut iaddr = self.get_register_i() as usize;
                for xi in 0..=x {
                    self.set_memory(iaddr, self.get_register(xi)?)?;
                    iaddr += 1;
                }
                self.set_register_i(iaddr as u16)?;
                self.advance()?;
            },
            Restore(x) => {
                let mut iaddr = self.get_register_i() as usize;
                for xi in 0..=x {
                    self.set_register(xi, self.get_memory(iaddr)?)?;
                    iaddr += 1;
                }
                self.set_register_i(iaddr as u16)?;
                self.advance()?;
            },
            Unknown(c) => {
                warn!("(UnknownOp)[pc|{:x}] {:0>4x}", self.pc, c);
                self.advance()?;
            },
        };

        self.trace_machine()?;

        Ok(())
    }

}
