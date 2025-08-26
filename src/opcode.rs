use std::fmt;

#[derive(Clone)]
pub enum Operation {
    /// 0NNN | Call
    /// call machine code routine at address NNN
    CallSysC(u16),

    /// 00E0 | Display
    /// clear the screen
    Clear,

    /// 00EE | Flow
    /// return from a subroutine
    Return,

    /// 1NNN | Flow
    /// jump to address NNN
    JumpC(u16),

    /// 2NNN | Flow
    /// call subroutine at NNN
    CallC(u16),

    /// 3xNN | Cond
    /// Skips the next instruction if VX equals NN
    SkipEqC(u8, u8),

    /// 4XNN | Cond
    /// Skips the next instruction if VX does not equal NN
    SkipNeC(u8, u8),

    /// 5XY0 | Cond
    /// Skips the next instruction if VX equals VY
    SkipEq(u8, u8),

    /// 6XNN | Const
    /// Sets VX to NN
    SetC(u8, u8),

    /// 7XNN | Const
    /// Add NN to VX
    AddC(u8, u8),

    /// 8XY0 | Assign
    /// Sets Vx to the value of VY
    Set(u8, u8),

    /// 8XY1 | BitOp | Vx |= Vy
    /// Set Vx to Vx or Vy
    Or(u8, u8),

    /// 8XY2 | BitOp | Vx &= Vy
    /// Set Vx to Vx and Vy
    And(u8, u8),

    /// 8XY3 | BitOp | Vx ^= Vy
    /// Set Vx to Vx xor Vy
    Xor(u8, u8),

    /// 8XY4 | Math | Vx += Vy
    /// Adds VY to VX. VF is set to 1 when there's an overflow, and to 0 when there is not.
    Add(u8, u8),

    /// 8XY5 | Math | Vx -= Vy
    /// VY is subtracted from VX. VF is set to 0 when there's an underflow, and 1 when there is not. (i.e. VF set to 1 if VX >= VY and 0 if not)
    Sub(u8, u8),

    /// 8XY6 | BitOp | Vx >>= Vy
    /// Shifts VX to the right by Vy, then stores the least significant bit of VX prior to the shift into VF.
    Shr(u8, u8),

    /// 8XY7 | Math | Vx = Vy - Vx
    /// Sets VX to VY minus VX. VF is set to 0 when there's an underflow, and 1 when there is not. (i.e. VF set to 1 if VY >= VX).[23]
    SubRev(u8, u8),

    /// 8XYE | BitOp | Vx <<= Vy
    /// Shifts Vx to the left by Vy, then sets VF to 1 if the most significant bit of VX prior to that shift was set, or to 0 if it was unset.[b][23]
    Shl(u8, u8),

    /// 9XY0 | Cond | if (Vx != Vy)
    /// Skips the next instruction if VX does not equal VY. (Usually the next instruction is a jump to skip a code block).[23]
    SkipNe(u8, u8),

    /// ANNN | MEM | I = NNN
    /// Sets I to the address NNN
    SetIC(u16),

    /// BNNN | Flow | PC = V0 + NNN
    /// Jumps to the address NNN plus V0
    JumpV0C(u16),

    /// CXNN | Rand | Vx = rand() & NN
    /// Sets VX to the result of a bitwise AND operation on a random number (Typically: 0 to 255) and NN
    RandC(u8, u8),

    /// DXYN | Display | draw(Vx, Vy, N)
    /// Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N pixels. Each row of 8 pixels is read as bit-coded starting from memory location I; I value does not change after the execution of this instruction. As described above, VF is set to 1 if any screen pixels are flipped from set to unset when the sprite is drawn, and to 0 if that does not happen
    DrawC(u8, u8, u8),

    /// EX9E | KeyOp | if (key() == Vx)
    /// Skips the next instruction if the key stored in VX(only consider the lowest nibble) is pressed (usually the next instruction is a jump to skip a code block)
    SkipEqKey(u8),

    /// EXA1 | KeyOp | if (key() != Vx)
    /// Skips the next instruction if the key stored in VX(only consider the lowest nibble) is not pressed (usually the next instruction is a jump to skip a code block).[23]
    SkipNeKey(u8),

    /// FX07 | Timer | Vx = get_delay()
    /// Sets VX to the value of the delay timer
    GetDelayTimer(u8),

    /// FX0A | KeyOp | Vx = get_key()
    /// A key press is awaited, and then stored in VX (blocking operation, all instruction halted until next key event, delay and sound timers should continue processing).[23]
    GetKey(u8),

    /// FX15 | Timer | delay_timer(Vx)
    /// Sets the delay timer to VX
    SetDelayTimer(u8),

    /// FX18 | Sound | sound_timer(Vx)
    /// Sets the sound timer to VX.[23]
    SetSoundTimer(u8),

    /// FX1E | MEM | I += Vx
    /// Adds VX to I. VF is not affected.
    AddI(u8),

    /// FX29 | MEM | I = sprite_addr[Vx]
    /// Sets I to the location of the sprite for the character in VX(only consider the lowest nibble). Characters 0-F (in hexadecimal) are represented by a 4x5 font.[23]
    SetIFont(u8),

    /// FX33 | BCD
    /// set_BCD(Vx)
    /// *(I+0) = BCD(3);
    /// *(I+1) = BCD(2);
    /// *(I+2) = BCD(1);
    /// Stores the binary-coded decimal representation of VX, with the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.[23]
    Bcd(u8),

    /// FX55 | MEM | reg_dump(Vx, &I)
    /// Stores from V0 to VX (including VX) in memory, starting at address I. The offset from I is increased by 1 for each value written, but I itself is left unmodified.[d][23]
    Store(u8),

    /// FX65 | MEM | reg_load(Vx, &I)
    /// Fills from V0 to VX (including VX) with values from memory, starting at address I. The offset from I is increased by 1 for each value read, but I itself is left unmodified.[d][23]
    Restore(u8),

    /// Unkown opcode | Fallback
    Unknown(u16),
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Operation::*;
        match self {
            CallSysC(addr) => write!(f, "call_sys {:0>12x}", addr),
            Clear => write!(f, "cls"),
            Return => write!(f, "ret"),
            JumpC(addr) => write!(f, "jmp #{:x}", addr),
            CallC(addr) => write!(f, "call #{:x}", addr),
            SkipEqC(x, c) => write!(f, "skp_eq v{:x}, #{:x}", x, c),
            SkipNeC(x, c) => write!(f, "skp_ne v{:x}, #{:x}", x, c),
            SkipEq(x, y) => write!(f, "skp_eq v{:x}, v{:x}", x, y),
            SetC(x, c) => write!(f, "set v{:x}, #{:x}", x, c),
            AddC(x, c) => write!(f, "add v{:x}, #{:x}", x, c),
            Set(x, y) => write!(f, "set v{:x}, v{:x}", x, y),
            Or(x, y) => write!(f, "or v{:x} v{:x}", x, y),
            And(x, y) => write!(f, "and v{:x}, v{:x}", x, y),
            Xor(x, y) => write!(f, "xor v{:x}, v{:x}", x, y),
            Add(x, y) => write!(f, "add v{:x}, v{:x}", x, y),
            Sub(x, y) => write!(f, "sub v{:x}, v{:x}", x, y),
            Shr(x, y) => write!(f, "rshf v{:x}, v{:x}", x, y),
            SubRev(x, y) => write!(f, "sub_neg v{:x}, v{:x}", x, y),
            Shl(x, y) => write!(f, "lshf v{:x}, v{:x}", x, y),
            SkipNe(x, y) => write!(f, "skp_ne v{:x}, v{:x}", x, y),
            SetIC(c) => write!(f, "set vI, #{:x}", c),
            JumpV0C(c) => write!(f, "jmp vI(#{:x})", c),
            RandC(x, c) => write!(f, "rand v{:x}, #{:x}", x, c),
            DrawC(x, y, c) => write!(f, "draw v{:x}, v{:x}, #{:x}", x, y, c),
            SkipEqKey(x) => write!(f, "skp_eq v{:x}, $key", x),
            SkipNeKey(x) => write!(f, "skip_ne v{:x}, $key", x),
            GetDelayTimer(x) => write!(f, "set v{:x}, $dtm", x),
            GetKey(x) => write!(f, "set v{:x}, $key", x),
            SetDelayTimer(x) => write!(f, "set $dtm, v{:x}", x),
            SetSoundTimer(x) => write!(f, "set $stm, v{:x}", x),
            AddI(x) => write!(f, "add vI, v{:x}", x),
            SetIFont(x) => write!(f, "set vI, Sprite(V{:x})", x),
            Bcd(x) => write!(f, "bcd v{:x}", x),
            Store(x) => write!(f, "store v{:x}", x),
            Restore(x) => write!(f, "restore v{:x}", x),
            Unknown(c) => write!(f, "unk(#{:0>4x})", c),
        }
    }
}

pub fn parse_opcode(opcode: u16) -> Operation {
    use Operation::*;
    let op_category = op0(opcode);
    let opcode = match op_category {
        0x0 => match oph1(opcode) {
            0xEE => Return,
            0xE0 => Clear,
            _ => CallSysC(opcode & 0xFFF),
        },
        0x1 => JumpC(opcode & 0xFFF),
        0x2 => CallC(opcode & 0xFFF),
        0x3 => SkipEqC(op1(opcode), oph1(opcode)),
        0x4 => SkipNeC(op1(opcode), oph1(opcode)),
        0x5 => SkipEq(op1(opcode), op2(opcode)),
        0x6 => SetC(op1(opcode), oph1(opcode)),
        0x7 => AddC(op1(opcode), oph1(opcode)),
        0x8 => match op3(opcode) {
            0 => Set(op1(opcode), op2(opcode)),
            1 => Or(op1(opcode), op2(opcode)),
            2 => And(op1(opcode), op2(opcode)),
            3 => Xor(op1(opcode), op2(opcode)),
            4 => Add(op1(opcode), op2(opcode)),
            5 => Sub(op1(opcode), op2(opcode)),
            6 => Shr(op1(opcode), op2(opcode)),
            7 => SubRev(op1(opcode), op2(opcode)),
            0xE => Shl(op1(opcode), op2(opcode)),
            _ => Unknown(opcode),
        },
        0x9 => {
            if op3(opcode) == 0 {
                SkipNe(op1(opcode), op2(opcode))
            } else {
                Unknown(opcode)
            }
        }
        0xA => SetIC(opcode & 0xFFF),
        0xB => JumpV0C(opcode & 0xFFF),
        0xC => RandC(op1(opcode), oph1(opcode)),
        0xD => DrawC(op1(opcode), op2(opcode), op3(opcode)),
        0xE => match oph1(opcode) {
            0x9E => SkipEqKey(op1(opcode)),
            0xA1 => SkipNeKey(op1(opcode)),
            _ => Unknown(opcode),
        },
        0xF => match oph1(opcode) {
            0x07 => GetDelayTimer(op1(opcode)),
            0x0A => GetKey(op1(opcode)),
            0x15 => SetDelayTimer(op1(opcode)),
            0x18 => SetSoundTimer(op1(opcode)),
            0x1E => AddI(op1(opcode)),
            0x29 => SetIFont(op1(opcode)),
            0x33 => Bcd(op1(opcode)),
            0x55 => Store(op1(opcode)),
            0x65 => Restore(op1(opcode)),
            _ => Unknown(opcode),
        },
        _ => Unknown(opcode),
    };
    opcode
}


fn oph1(opcode: u16) -> u8 {
    (opcode & 0xFF) as u8
}

fn op0(opcode: u16) -> u8 {
    ((opcode & 0xF000) >> 12) as u8
}

fn op1(opcode: u16) -> u8 {
    ((opcode & 0x0F00) >> 8) as u8
}

fn op2(opcode: u16) -> u8 {
    ((opcode & 0x00F0) >> 4) as u8
}

fn op3(opcode: u16) -> u8 {
    (opcode & 0x000F) as u8
}
