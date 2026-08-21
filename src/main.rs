use std::fs::File;
use std::io::prelude::*;
use console_engine::Color;
use console_engine::KeyCode;
use console_engine::pixel;

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

struct Chip8 {
    memory: [u8; 4096],
    screen: [bool; SCREEN_WIDTH*SCREEN_HEIGHT],
    pc: u16,
    i: u16,
    stack: [u16; 16],
    v: [u8; 16],
    sp: u16,
    dl: u8,
    sl: u8,
}

fn slip_num(x: u8) -> Vec<u8> {
    let mut number = x;
    if number == 0 {
        return vec![0];
    }
    
    let mut digitos = Vec::new();
    while number > 0 {
        digitos.push(number % 10);
        number /= 10;
    }
    
    digitos.reverse();
    digitos
}

impl Chip8 {
    fn new() -> Chip8 {
        let mut new_chip: Chip8 = Chip8 {
            memory: [0; 4096],
            screen: [false; SCREEN_WIDTH*SCREEN_HEIGHT],
            pc: 0x200,
            i: 0,
            stack: [0; 16],
            v: [0; 16],
            sp: 0,
            dl: 0,
            sl: 0,
        };

        new_chip.memory[0x050..0x09f+1].copy_from_slice(&FONT);

        new_chip
    }

    // Function about Stack
    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }
    fn del(&mut self) -> u16 {
        let mut pc: u16 = 0;
        pc = self.stack[self.sp as usize];
        self.stack[self.sp as usize] = 0x0;
        self.sp -= 1;

        pc
    }
    
    // Load ROM in RAM
    fn load_archive(&mut self, path: String) {
        let mut file = File::open(path);
        let mut buffer = Vec::new();
        file.expect("fallo").read_to_end(&mut buffer);

        let start = 0x200 as usize;
        let end = (0x200 as usize) + buffer.len();

        self.memory[start..end].copy_from_slice(&buffer);
    }

    fn fetch(&mut self) -> u16 {
        let bit_left = self.memory[self.pc as usize] as u16;
        let bit_right = self.memory[(self.pc+1) as usize] as u16;

        let op = (bit_left << 8) | bit_right;
        self.pc += 2;

        op
    }

    fn clear_screen(&mut self) {
        self.screen = [false; SCREEN_WIDTH*SCREEN_HEIGHT];
    }

    fn jump(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    fn jump_stack(&mut self, nnn: u16) {
        self.push(self.pc);
        self.pc = nnn;
    }

    fn del_stack(&mut self) {
        self.pc = self.del();
    }

    // Skip instruccion
    fn skip_3xnn(&mut self, x: u8, nn: u8) {
        if x == nn {
            self.pc += 2;
        }
    }
    
    fn skip_4xnn(&mut self, x: u8, nn: u8) {
        if self.v[x as usize] != nn {
            self.pc += 2;
        }
    }

    fn skip_5xy0(&mut self, x: u8, y: u8) {
        if self.v[x as usize] == self.v[y as usize] {
            self.pc += 2;
        }
    }

    fn skip_9xy0(&mut self, x: u8, y: u8) {
        if self.v[x as usize] != self.v[y as usize] {
            self.pc += 2;
        }
    }
    
    // ---Instruccion 8XY---
    
    fn set_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
    }
    
    fn operator_or(&mut self, x: u8, y: u8) {
        self.v[x as usize] |= self.v[y as usize];
    }
    
    fn operator_and(&mut self, x: u8, y: u8) {
        self.v[x as usize] &= self.v[y as usize];
    }
    
    fn operator_xor(&mut self, x: u8, y: u8) {
        self.v[x as usize] ^= self.v[y as usize];
    }
    
    fn add_vx_vy(&mut self, x: u8, y: u8) {
        
        let (sum, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
        
        self.v[x as usize] = sum;
        self.v[0xF] = if overflow { 1 } else { 0 };
    }
    
    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        let flag = if self.v[x as usize] >= self.v[y as usize] { 1 } else { 0 };
        
        self.v[x as usize] = self.v[x as usize].wrapping_sub(self.v[y as usize]);
        self.v[0xF] = flag;
    }
    
    fn sub_vy_vx(&mut self, x: u8, y: u8) {
        let flag = if self.v[y as usize] >= self.v[x as usize] { 1 } else { 0 };
        
        self.v[x as usize] = self.v[y as usize] - self.v[x as usize];
        self.v[0xF] = flag;
    }
    
    fn shift_right_vx(&mut self, x: u8) {
        self.v[0xF] = self.v[x as usize] & 0x1;
        self.v[x as usize] = self.v[x as usize] >> 1;
    }
    
    fn shift_left_vx(&mut self, x: u8) {
        self.v[0xF] = (self.v[x as usize] >> 7) & 0x1;
        self.v[x as usize] = self.v[x as usize] << 1;
    }
    // ---------------------
    
    fn set(&mut self, x: u8, nn: u8) {
        self.v[x as usize] = nn;
    }

    fn add(&mut self, x: u8, nn: u8) {
        self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
    }

    fn set_index(&mut self, nnn: u16) {
        self.i = nnn;
    }

    fn display(&mut self, vx: usize, vy: usize, n: u8) {

        let x = vx as u16;
        let y = vy as u16;

        let num = n;
        
        self.v[0xf] = 0;

        let mut flag = false;

        for rows in 0..num {
            let byte = self.memory[(self.i as usize) + (rows as u16) as usize];

            for row in 0..8 {

                if (byte & (0x80 >> row)) != 0 {
                    let x_coord = (x + (row as u16)) as usize % SCREEN_WIDTH;
                    let y_coord = (y + (rows as u16)) as usize % SCREEN_HEIGHT;

                    let coord = x_coord + SCREEN_WIDTH * y_coord;

                    flag |= self.screen[coord];
                    self.screen[coord] ^= true;
                }

            }

            self.v[0xf] = if flag { 1 } else { 0 };
        }
    }
    
    // ---- fx00 ----
    
    fn save_memory(&mut self, x: u8) {
        for i in 0..(x as usize) {
            self.memory[(self.i as usize) + i] = self.v[i];
        }
    }
    
    fn load_memory(&mut self, x: u8) {
        for i in 0..(x as usize) {
            self.v[i] = self.memory[(self.i as usize) + i];
        }
    }
    
    fn div_vx(&mut self, x: u8) {
        let list = slip_num(x);
        
        for i in 0..(list.len() - 1) {
            self.memory[(i as usize) + i] = list[i];
        }
    }
    
    fn add_i(&mut self, x: u8) {
        self.i += self.v[x as usize] as u16;
    }
    // --------------
    
    fn run(&mut self, opcode: u16) {
        let mut x: u8 = ((opcode >> 8) & 0x000f).try_into().unwrap();
        let mut y: u8 = ((opcode >> 4) & 0x000f).try_into().unwrap();
        let mut n: u8 = (opcode & 0xf).try_into().unwrap();
        let mut nn: u8 = (opcode & 0xff).try_into().unwrap();
        let mut nnn: u16 = opcode & 0xfff;

        let init = opcode & 0xf000;
        
        match opcode {
            0x00e0 => self.clear_screen(),
            _ => match init {
                0x1000 => self.jump(nnn),
                0x2000 => self.jump_stack(nnn),
                0x00ee => self.del_stack(),
                0x3000 => self.skip_3xnn(x, nn),
                0x4000 => self.skip_4xnn(x, nn),
                0x5000 => self.skip_5xy0(x, y),
                0x6000 => self.set(x, nn),
                0x7000 => self.add(x, nn),
                0x8000 => {match n {
                            0x0 => self.set_vx_vy(x, y),
                            0x1 => self.operator_or(x, y),
                            0x2 => self.operator_and(x, y),
                            0x3 => self.operator_xor(x, y),
                            0x4 => self.add_vx_vy(x, y),
                            0x5 => self.sub_vx_vy(x, y),
                            0x6 => self.shift_right_vx(x),
                            0x7 => self.sub_vy_vx(x, y),
                            0xE => self.shift_left_vx(x),
                            1_u8..=u8::MAX => todo!(),
                };},
                0x9000 => self.skip_9xy0(x, y),
                0xa000 => self.set_index(nnn),
                0xd000 => self.display(self.v[x as usize].into(), self.v[y as usize].into(), n),
                0xf000 => {match (nn){
                            0x33 => self.div_vx(self.v[x as usize]),
                            0x55 => self.save_memory(x),
                            0x65 => self.load_memory(x),
                            1_u8..=u8::MAX => todo!(),
                            0_u8 => todo!(),
                };},
                _ => println!("no hay instruccion"),
            }
        }    
    }
}


fn main() {
    let mut x: Chip8 = Chip8::new();

    let mut engine = console_engine::ConsoleEngine::init(64, 32, 3);
    
    x.load_archive("./roms/test3.ch8".to_string());

    loop {
        engine.as_mut().expect("FALLO").wait_frame();
        engine.as_mut().expect("FALLO").clear_screen();
        let op = x.fetch();

        x.run(op);
        
        for (i, pixel) in x.screen.iter().enumerate() {
            if *pixel {
                /*engine.as_mut().expect("FALLO").print(0, 0, format!("{} {}", *pixel, i).as_str());*/
            
                let x = (i % SCREEN_WIDTH) as u32;
                let y = (i / SCREEN_WIDTH) as u32;

                engine.as_mut().expect("FALLO").set_pxl(x as i32, y as i32, pixel::pxl_fg('O', Color::Green));
            }
        }

        if engine.as_mut().expect("FALLO").is_key_pressed(KeyCode::Char('q')) {
            break;
        }

        engine.as_mut().expect("FALLO").draw();
    }

}