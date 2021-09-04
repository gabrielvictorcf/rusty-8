# rusty-8 💾
Just a GL [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) emulator written in Rust 💾

<div align="center">
    <img src="https://i.imgur.com/L5XbjPf.png" alt="rusty-8 running MAZE rom">
    <br>
    <p align="center">
    <em>rusty-8 running the maze ROM</em>
    </p>
    <br>
</div>

Chip-8 is a virtual machine and easy-to-use interpreted programming language from the late 70's.
Typically, the roms are in a compiled form - which is what this emulator executes. Since it wasn't
accompanied by dedicated hardware, there are specifications on all the components a Chip-8
implementation should have.

This project was made after i had *Computer Architecture and Organization* classes, a pretty cool
subject that made me want to get my hands dirty with emulation.

With this in mind, if you're curious about those topics
[take a look at this repo's resources!](https://github.com/gabrielvictorcf/rusty-8#credits)
It's more hands-on approach, but there is definitely something to be learned, like how a computer
executes instructions, branches execution, and more.

Also, the code is thoroughly commented, and should be easy to follow if you're familiar with rust.
The only caveat is the `fb.glutin_handle_basic_input` function, at the end of `main`, which is heavy library
code - `input` is just a recap of a frame's input events from OpenGL.

Here are some specs of my Chip-8 implementation:
- 500 Hz Clock
- 4Kb of RAM memory (512 bytes reserved to the Virtual Machine)
- 64x32 display (resizeable with OpenGl)
- 16 8-bit data registers, plus some other special ones
- 60Hz playback rate

## Installing / Building
> You'll need `git` and `cargo` in your machine.
>
> If you're a Windows user, you might want to fix the rom path with a backslash.

```bash
git clone https://github.com/gabrielvictorcf/rusty-8.git
cd rusty-8
cargo run --release -- roms/<rom_of_your_choice> # This should open the window with the emulator!
```

## Running
```bash
# Assuming you're on /rusty-8
cargo run --release -- roms/<rom_of_your_choice> # You can run with cargo

# OR

./target/release/rusty_8 <rom_path> # Or call the binary directly
```

Some games might be buggy - this is unrelated to the emulator itself, and depends more on
how the game was programmed. Also, if you find the window a bit large, it's resizeable.

## Controls
Chip-8's original keyboard and this emulator's keyboards are as follows:
```
    Original                     Rusty-8
╔═══╦═══╦═══╦═══╗            ╔═══╦═══╦═══╦═══╗
║ 1 ║ 2 ║ 3 ║ C ║            ║ 1 ║ 2 ║ 3 ║ 4 ║
╠═══╬═══╬═══╬═══╣            ╠═══╬═══╬═══╬═══╣
║ 4 ║ 5 ║ 6 ║ D ║            ║ Q ║ W ║ E ║ R ║
╠═══╬═══╬═══╬═══╣     →      ╠═══╬═══╬═══╬═══╣
║ 7 ║ 8 ║ 9 ║ E ║            ║ A ║ S ║ D ║ F ║
╠═══╬═══╬═══╬═══╣            ╠═══╬═══╬═══╬═══╣
║ A ║ 0 ║ B ║ F ║            ║ Z ║ X ║ C ║ V ║
╚═══╩═══╩═══╩═══╝            ╚═══╩═══╩═══╩═══╝
```

There are also some additional emulator/window controls:
- Window close - `Esc` or `Ctrl+W`
- Emulator reset - `Ctrl+R`

## Credits
All of these are amazing, free, resources that make learning/implementing Chip-8 quite a pleasure.
Big thanks to all of these creators!

- Matt Mikolay's [Chip-8 Technical Reference](https://github.com/mattmikolay/chip-8/wiki/CHIP%E2%80%908-Technical-Reference)
- Cowgod's [Chip-8 Technical Reference](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM)
- [Test rom](https://github.com/corax89/chip8-test-rom)
- [Game roms](https://github.com/kripod/chip8-roms)

More game roms shouldn't be hard to find around the internet.

This was built using some nice crates, so thanks to their creators as well.

## License
All code in this repository is licensed under the MIT License, and thus is of free use.