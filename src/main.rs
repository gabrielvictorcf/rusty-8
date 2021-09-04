use mini_gl_fb::{self, config};
use mini_gl_fb::glutin::{dpi::LogicalSize, event::VirtualKeyCode};
use rodio::{OutputStream, Source, source::SineWave};
use std::time::{Duration, Instant};

mod chip8;

use chip8::Chip8;
use chip8::{SCREEN_WIDTH, SCREEN_HEIGHT};

const SCREEN_SCALE: usize = 8;      // Initial scale between Chip-8 screen and displayed Window
const WINDOW_WIDTH:  f64  = (SCREEN_WIDTH  * SCREEN_SCALE) as f64;  // Displayed Window Width
const WINDOW_HEIGHT: f64  = (SCREEN_HEIGHT * SCREEN_SCALE) as f64;  // Displayed Window Height

// Array mapping Key codes to keys in the chip8 keyboard
const CHIP8_VIRTUAL_KEY_CODES: [VirtualKeyCode; 16] = [
    VirtualKeyCode::X,      // 0
    VirtualKeyCode::Key1,   // 1
    VirtualKeyCode::Key2,   // 2
    VirtualKeyCode::Key3,   // 3
    VirtualKeyCode::Q,      // 4
    VirtualKeyCode::W,      // 5
    VirtualKeyCode::E,      // 6
    VirtualKeyCode::A,      // 7
    VirtualKeyCode::S,      // 8
    VirtualKeyCode::D,      // 9
    VirtualKeyCode::Z,      // A
    VirtualKeyCode::C,      // B
    VirtualKeyCode::Key4,   // C
    VirtualKeyCode::R,      // D
    VirtualKeyCode::F,      // E
    VirtualKeyCode::V       // F
];

/// Read keys that are down during `input` event poll.
fn read_chip8_keys(keyboard: &mut [bool; 16], input: &mini_gl_fb::BasicInput) {
    for (key_pos, key_code) in CHIP8_VIRTUAL_KEY_CODES.iter().enumerate() {
        keyboard[key_pos] = input.key_is_down(*key_code);
    }
}

fn main() {
    let rom = match std::env::args().nth(1) {
        Some(rom) => rom,
        None => {
            eprintln!("Missing rom file path. Try ./rusty8 <rom_path> or cargo run --release -- <rom_path>");
            std::process::exit(1);
        },
    };

    let mut chip8 = Chip8::new();
    if let Err(e) = chip8.load_rom(rom) {
        eprintln!("Failure during ROM open/read\n{}", e);
        std::process::exit(1);
    }

    // Initializing window - event loop and config
    let mut event_loop = mini_gl_fb::glutin::event_loop::EventLoop::new();
    let config = config! {
        window_title: String::from("rusty-8"),
        window_size: LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        buffer_size: Some(LogicalSize::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)),
        resizable: true,
        invert_y: false
    };
    
    // Initializing window - create framebuffer, set it to B&W and paint blank screen
    let mut fb = mini_gl_fb::get_fancy(config, &event_loop);
    fb.change_buffer_format::<u8>(mini_gl_fb::BufferFormat::R);
    fb.use_grayscale_shader();
    fb.update_buffer(&chip8.screen);

    // Get handle to audio device, create audio source then make audio controller
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let source = SineWave::new(815).take_duration(Duration::from_millis(12));
    let beep = rodio::Sink::try_new(&handle).unwrap();
    beep.set_volume(0.3);

    // Event loop helpers - callback ids and playback sound
    let mut timers_id = None;
    let mut tick_id = None;

    fb.glutin_handle_basic_input(&mut event_loop, |fb, input| {
        read_chip8_keys(&mut chip8.keyboard, &input);

        let mut should_close = input.key_is_down(VirtualKeyCode::Escape);
        should_close |= chip8.finished_running();
        should_close |= input.key_is_down(VirtualKeyCode::LControl) && input.key_is_down(VirtualKeyCode::W);

        if should_close { // Exit event loop and close program
            return false;
        }

        if input.resized { // Redraw window to accomodate for new viewport
            fb.redraw();
        }

        let should_reboot = input.key_is_down(VirtualKeyCode::LControl) && input.key_is_down(VirtualKeyCode::R);
        if should_reboot { // Reboot the chip8 with current ROM
            chip8.reboot();
            fb.update_buffer(&chip8.screen);
        }

        // Special handling needed when chip8 is idly waiting for a key press
        if chip8.waiting.is_some() {
            match chip8.keyboard.iter().position(|&key_down| key_down == true) {
                Some(key_pos) => { // Answer which key was pressed and reset loop events
                    chip8.answer_key(key_pos as u8);
                    
                    input.wait = false;
                    timers_id = None;
                    tick_id = None;
                },
                None => { // Clear event loop and kick into waiting mode
                    input.wait = true;
                    if !input.wakeups.is_empty() {
                        input.wakeups.clear();
                    }

                    return true; // While chip8 isn't answered, ignore events
                }
            }
        }

        // ---- Event handling ----
        // Inserting events in the event queue
        if let None = timers_id {
            timers_id = Some(input.schedule_wakeup(Instant::now()));
        }
        
        if let None = tick_id {
            tick_id = Some(input.schedule_wakeup(Instant::now()));
        }

        // Executing event, if there is any.
        if let Some(mut wakeup) = input.wakeup {
            if Some(wakeup.id) == tick_id { // Tick one clock cycle of the chip8
                chip8.tick();

                if chip8.screen_updated {
                    fb.update_buffer(&chip8.screen);
                }

                wakeup.trigger_after(Duration::from_millis(2));
                input.reschedule_wakeup(wakeup);
            } else if Some(wakeup.id) == timers_id { // Tick chip8 timers on 60Hz
                let is_beeping = chip8.tick_timers();
                if is_beeping {
                    beep.append(source.clone());
                    beep.play()
                }

                wakeup.trigger_after(Duration::from_millis(16));
                input.reschedule_wakeup(wakeup);
            }
        }
        
        true
    });
}
