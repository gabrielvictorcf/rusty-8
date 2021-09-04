use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use nanorand::Rng;
use std::path::Path;

// The original CHIP-8 interpreter occupies the first 512 bytes.
// That way, every program starts at byte offset 0x200 (512).
// All instructions are 2 bytes long and are stored most-significant-byte first (BIG ENDIAN);
// instructions must be even aligned, so sprites may need to pad the RAM to guarantee this

const MEM_SIZE:    usize = 4096;    // 4Kb of RAM (address range = 0x000 to 0xFFF).
const PROG_OFFSET: usize   = 0x200;   // ROM's are loaded on addr. 0x200.

const STACK_START: u8    = 0x000;   // Stack is the first 0x100 bytes of memory.
const STACK_END:   u8    = 0x0FF;

const SPRITES_START: usize = 0x0FF;   // Sprites start right after stack.
const SPRITES_END:   usize = 0x14F;   // Sprites end right before program offset.

pub const SCREEN_WIDTH:  usize = 64;    // Internal Chip-8 Screen Width
pub const SCREEN_HEIGHT: usize = 32;    // Internal Chip-8 Screen Height

const CHIP8_FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0,   // 0
    0x20, 0x60, 0x20, 0x20, 0x70,   // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0,   // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0,   // 3
    0x90, 0x90, 0xF0, 0x10, 0x10,   // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0,   // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0,   // 6
    0xF0, 0x10, 0x20, 0x40, 0x40,   // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0,   // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0,   // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90,   // a
    0xE0, 0x90, 0xE0, 0x90, 0xE0,   // b
    0xF0, 0x80, 0x80, 0x80, 0xF0,   // c
    0xE0, 0x90, 0x90, 0x90, 0xE0,   // d
    0xF0, 0x80, 0xF0, 0x80, 0xF0,   // e
    0xF0, 0x80, 0xF0, 0x80, 0x80    // f
];

type Nibbles = (usize,usize,usize,usize);

fn decode(opcode: u16) -> Nibbles {
    let nibble_1 = ((opcode & 0xF000) >> 12) as usize;
    let nibble_2 = ((opcode & 0x0F00) >>  8) as usize;
    let nibble_3 = ((opcode & 0x00F0) >>  4) as usize;
    let nibble_4 = ((opcode & 0x000F) >>  0) as usize;

    return (nibble_1, nibble_2, nibble_3, nibble_4);
}

pub struct Chip8 {
    memory: [u8; MEM_SIZE],
    memory_end: usize,
    v:  [u8; 16],   // General purpose Vx registers. VF is special flag register.
    i:  u16,        // Index register
    pc: u16,        // Program-counter
    sp: u8,         // Stack pointer
    dt: u8,         // Delay timer register
    st: u8,         // Sound timer register
    pub keyboard: [bool; 16],   // Keyboard with keys' state (up | down) -> keys from 0x0 to 0xF
    pub waiting: Option<u8>,    // Index [0..F] of register waiting for a keypress
    pub screen: Vec<u8>,        // Internal screen buffer
    pub screen_updated: bool    // Screen was updated in last tick 
}

impl Chip8 {
    pub fn new() -> Self {
        // Initializes the whole memory to 0, then the font area
        let mut memory = [0; MEM_SIZE]; // Init the whole memory to 0
        memory[SPRITES_START..SPRITES_END].copy_from_slice(&CHIP8_FONT);

        Chip8 {
            memory,      
            memory_end: PROG_OFFSET,    // Marks the end of CHIP-8's loaded ROM
            v:      [0; 16],            // Init registers to 0
            i:      0,
            pc:     PROG_OFFSET as u16, // Program ROM offset in CHIP-8 RAM
            sp:     STACK_START,
            dt:     0,
            st:     0,
            keyboard: [false; 16],
            waiting: None,
            screen: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT],
            screen_updated: false
        }
    }
    
    pub fn load_rom<P: AsRef<Path>>(&mut self,rom: P) -> std::io::Result<()> {
        let mut rom = File::open(rom)?;
        self.memory_end += rom.read(&mut self.memory[(PROG_OFFSET as usize)..MEM_SIZE])?;

        Ok(())
    }
    
    pub fn reboot(&mut self) {
        // Reset registers
        self.v.fill(0);
        self.i = 0;
        self.dt = 0;
        self.st = 0;

        // Reset peripherals
        self.keyboard.fill(false);
        self.waiting = None;
        self.screen.fill(0);
        self.screen_updated = false;

        // Reset memory (stack and ram) and reboot program (program counter)
        self.memory[(STACK_START as usize)..(STACK_END as usize)].fill(0);
        self.sp = STACK_START;
        self.memory[self.memory_end..MEM_SIZE].fill(0);
        self.pc = PROG_OFFSET as u16;
    }

    // Function for debugging binary ROM data
    #[allow(dead_code)]
    pub fn dump_rom(&mut self) {
        for addr in (PROG_OFFSET..self.memory_end).step_by(2) {
            let instruction = self.fetch(addr);
            let opcode = decode(instruction);

            eprintln!("{:#03X}:\t{:04X}\t{:?}", addr, instruction, opcode);
        }
    }

    // Function for debugging internal processor data and states.
    #[allow(dead_code)]
    pub fn dump(&self) {
        let pc = self.pc as usize;
        let instruction = self.fetch(pc);
        let opcode = decode(instruction);
        eprintln!("{:#03X}:\t{:04X}\t{:?}", pc, instruction,opcode);

        eprint!("\t");
        for i in 0..8 {
            eprint!("v{:x}: {:02x} ", i, self.v[i]);
        }
        eprint!("\n\t");
        for i in 8..16 {
            eprint!("v{:x}: {:02x} ", i, self.v[i]);
        }
        eprint!("\n");

        eprintln!("\ti: {:X}", self.i);
        eprintln!("\tsp: {:X}", self.sp);
        eprintln!("\tdt: {:X}", self.dt);
        eprintln!("\tst: {:X}", self.st);
    }

    fn fetch(&self,addr: usize) -> u16 {
        if addr < PROG_OFFSET || addr >= self.memory_end {
            eprintln!("Invalid memory access at address {}.",addr);
            std::process::exit(1);
        }

        // Safely unwrapping because bounds are checked above
        u16::from_be_bytes(self.memory[addr..addr+2].try_into().unwrap())
    }

    /// Will update the internal chip8 timers, if they need to.
    /// Returns true if a sound should be played after this tick.
    pub fn tick_timers(&mut self) -> bool {
        if self.dt > 0 {
            self.dt -= 1;
        }
        
        if self.st > 0 {
            self.st -= 1;
            return true;
        }

        false
    }

    /// Answer the chip8 with the first key pressed.
    /// After this, chip8 will no longer be waiting.
    pub fn answer_key(&mut self, key_pos: u8) {
        if let Some(reg_index) = self.waiting.take() {
            self.v[reg_index as usize] = key_pos;
        }
    }

    /// Query wether chip8 is done executing.
    /// Returns true if program counter is at loaded ROM's end.
    pub fn finished_running(&self) -> bool {
        self.pc as usize == self.memory_end
    }

    /// Process a single cycle of chip8's loaded rom.
    /// Will exit the program if an invalid memory address is reached.
    /// # Panics
    /// Panics if an invalid (unknown) instruction is decoded.
    pub fn tick(&mut self) {
        // 0. Internal state updating
        self.screen_updated = false;

        // 1. Instruction Fetch
        let instruction = self.fetch(self.pc as usize);
        self.pc += 2;	// Increment pc for next instruction

        // 2. Instruction Decode
		let nibbles = decode(instruction);			// Instruction decoded in 4 4-bit groups
        let address = instruction & 0x0FFF;			// nnn / addr
        let byte = (instruction & 0x00FF) as u8;	// kk  / byte
        let nibble = nibbles.3;						// n   / nibble
        let y = nibbles.2; 							// x   / Index of Vy register
        let x = nibbles.1; 							// y   / Index of Vx register
        match nibbles {
            (0x0, 0x0, 0xE, 0x0) => { // CLS - Clear the whole display (set all pixels to 0)
                self.screen.fill(0);
            }
            (0x0, 0x0, 0xE, 0xE) => { // RET - Pop address in top of stack and jump to it
                self.sp -= 2;

                let sp = self.sp as usize;
                self.pc = u16::from_le_bytes(self.memory[sp..sp+2].try_into().unwrap());
            }
            (0x1, _, _, _) => { // JP addr - Jump to address
                self.pc = address;
            },
            (0x2, _, _, _) => { // CALL addr - Push pc then jump to addr
                let sp = self.sp as usize;
                self.memory[sp..sp+2].copy_from_slice(&self.pc.to_le_bytes());
                self.sp += 2;

                self.pc = address;
            },
            (0x3, _, _, _) => { // SE Vx, kk - Skip next instruction if V[x] == kk (byte)
                if self.v[x] == byte {
                    self.pc += 2;
                }
            },
            (0x4, _, _, _) => { // SNE Vx, kk - Skip next instruction if V[x] != kk (byte)
                if self.v[x] != byte {
                    self.pc += 2;
                }
            },
            (0x5, _, _, _) => { // SE Vx, Vy - Skip next instruction if V[x] == V[y]
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            },
            (0x6, _, _, _) => { // LDI Vx, byte - Set register V[x] to kk (byte)
                self.v[x] = byte;
            },
            (0x7, _, _, _) => { // ADDI Vx, byte - Set V[x] = V[x] + kk (byte)
                self.v[x] = self.v[x].wrapping_add(byte);
            },
            (0x8, _, _, 0x0) => { // MOV Vx, Vy - Set V[x] = V[y]
                self.v[x] = self.v[y];
            },
            (0x8, _, _, 0x1) => { // OR Vx, Vy - Set V[x] = V[x] | V[y]
                self.v[x] = self.v[x] | self.v[y];
            },
            (0x8, _, _, 0x2) => { // AND Vx, Vy - Set V[x] = V[x] & V[y]
                self.v[x] = self.v[x] & self.v[y];
            },
            (0x8, _, _, 0x3) => { // XOR Vx, Vy - Set V[x] = V[x] ^ V[y]
                self.v[x] = self.v[x] ^ self.v[y];
            },
            (0x8, _, _, 0x4) => { // ADD Vx, Vy - Set V[x] = V[x] + V[y] -> Vf = 1 on carry
                let (vx, vy) = (self.v[x], self.v[y]);
                let (sum, is_overflowing) = vx.overflowing_add(vy);

                self.v[x] = sum;
                self.v[0xf] = is_overflowing as u8;
            },
            (0x8, _, _, 0x5) => { // SUB Vx, Vy - Set V[x] = V[x] - V[y] -> Vf = 0 on borrow
                let (vx, vy) = (self.v[x], self.v[y]);
                let (sub, is_borrowing) = vx.overflowing_sub(vy);

                self.v[x] = sub;
                self.v[0xf] = (!is_borrowing) as u8;
            },
            (0x8, _, _, 0x6) => { // SHR Vx - Right shift Vx by 1 -> Vf = 1 if least-significant bit is set
                let vx = self.v[x];
                let (rshifted, lsb) = (vx>>1, vx & 1);

                self.v[x] = rshifted;
                self.v[0xf] = lsb;
            },
            (0x8, _, _, 0x7) => { // SUB Vx, Vy - Set V[x] = V[x] - V[y] -> Vf = 0 on borrow
                let (vx, vy) = (self.v[x], self.v[y]);
                let (sub, is_borrowing) = vy.overflowing_sub(vx);

                self.v[x] = sub;
                self.v[0xf] = (!is_borrowing) as u8;
            },
            (0x8, _, _, 0xE) => { // SHL Vx - Left shift Vx by 1 -> Vf = 1 if most-significant bit is set
                let vx = self.v[x];
                let (lshifted, msb) = (vx<<1, (vx & 128)>>7);

                self.v[x] = lshifted;
                self.v[0xf] = msb;
            },
            (0x9, _, _, _) => { // SNE Vx, Vy - Skip next instruction if Vx != Vy
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            },
            (0xA, _, _, _) => { // LD I, nnn - Set register I to nnn
                self.i = address;
            },
            (0xB, _, _, _) => { // JP V0, addr - Jump to memory[addr + V[0]]
                self.pc = address + self.v[0] as u16;
            },
            (0xC, _, _, _) => { // RND Vx, kk - Set V[x] to random byte [0,255] AND kk (byte)
                let rand_byte = nanorand::tls_rng().generate::<u8>();
                self.v[x] = rand_byte & byte;
            },
            (0xD, _, _, _) => { // DRAW Vx, Vy, n - Draw n-length sprite at screen[x][y] - V[f] = 1 on collision
                let sprite_start = self.i as usize;
                let sprite_end = sprite_start + nibble;
                let sprite = &self.memory[sprite_start..sprite_end];
                
                let x_pos = self.v[x] as usize % 64;
                let mut y_pos = self.v[y] as usize % 32;
                let mut has_collided = 0; // No collision has occurred!
                for byte in sprite {
                    let sprite_pos = x_pos + y_pos * SCREEN_WIDTH;
                    let limit_pos = (y_pos+1) * SCREEN_WIDTH; // pos where this row ends

                    for bit_pos in 0..8 {
                        let pixel_pos = sprite_pos + bit_pos ;
                        if pixel_pos >= limit_pos { break; }; // clip sprites that wrap after row ended

                        let pixel = self.screen[pixel_pos] & 1;
                        let bit = (byte >> (7 - bit_pos)) & 1;

                        has_collided |= pixel & bit; // If pixel gets unset -> V[f] = 1 (pixel collision!)
                        self.screen[pixel_pos] = (pixel ^ bit) * 255; // Paint pixels on XOR mode
                    }

                    y_pos = (y_pos+1) % 32;
                }

                self.v[0xf] = has_collided;
                self.screen_updated = true;
            },
            (0xE, _, 0x9, 0xE) => { // SKP Vx - Skip next instruction if key[Vx] IS pressed (key is down)
                let vx = self.v[x];
                let is_key_pressed = self.keyboard[vx as usize];
                if is_key_pressed {
                    self.pc += 2;
                }
            },
            (0xE, _, 0xA, 0x1) => { // SKNP Vx - Skip next instruction if key[Vx] is NOT pressed (key is up)
                let vx = self.v[x];
                let is_key_pressed = self.keyboard[vx as usize];
                if is_key_pressed == false {
                    self.pc += 2;
                }
            },
            (0xF, _, 0x0, 0x7) => { // LD Vx, DT - Set V[x] = Delay timer
                self.v[x] = self.dt;
            },
            (0xF, _, 0x0, 0xA) => { // LD Vx, Key - Set V[x] = key
                self.keyboard.fill(false); // Clear keyboard state
                self.waiting = Some(x as u8);
            },
            (0xF, _, 0x1, 0x5) => { // LD DT, Vx - Set Delay timer = V[x]
                self.dt = self.v[x];
            },
            (0xF, _, 0x1, 0x8) => { // LD ST, Vx - Set Sound timer = V[x]
                self.st = self.v[x];
            },
            (0xF, _, 0x1, 0xE) => { // ADD I, Vx - Add Vx to register I
                self.i += self.v[x] as u16;
            },
            (0xF, _, 0x2, 0x9) => { // LD I, Sprite[Vx] - Set I to address of sprite Vx
                self.i = SPRITES_START as u16 + (self.v[x] * 5) as u16;
            },
            (0xF, _, 0x3, 0x3) => { // STORE BCD, Vx - Store BCD in memory[register I]
                // BCD = Binary-coded Decimal -> https://en.wikipedia.org/wiki/Binary-coded_decimal
                let vx = self.v[x];
                let hundreds = (vx / 100) % 10;
                let tenths = (vx / 10) % 10;
                let ones = vx % 10;

                self.memory[(self.i)   as usize] = hundreds;
                self.memory[(self.i+1) as usize] = tenths;
                self.memory[(self.i+2) as usize] = ones;
            },
            (0xF, _, 0x5, 0x5) => { // STORE MEM[I..I+x], V[0..x] - Store starting from reg v0 into mem[register I..I+x]
                let (start, end) = (self.i as usize, self.i as usize + x);
                self.memory[start..=end].copy_from_slice(&self.v[0..=x]);
                self.i += x as u16 + 1;
            },
            (0xF, _, 0x6, 0x5) => { // READ V[0..x], MEM[I..I+x] - Read starting from pos[register I] into v0..vx
                let (start, end) = (self.i as usize, self.i as usize + x);
                self.v[0..=x].copy_from_slice(&self.memory[start..=end]);
                self.i += x as u16 + 1;
            },
            _ => panic!("Instruction not specified: {:04X} -- decoded -> {:?}", instruction, nibbles)
        }
    }
}