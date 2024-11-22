mod handmade;

use std::mem;

use handmade::*;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::PixelFormatEnum;

const MOVE_UP: usize = 0;
const MOVE_DOWN: usize = 1;
const MOVE_LEFT: usize = 2;
const MOVE_RIGHT: usize = 3;

fn debug_platform_free_file_memory(_thread: &ThreadContext, _memory: &mut [u8]) {
    // Implement freeing file memory if necessary
}

fn debug_platform_read_entire_file(_thread: &ThreadContext, filename: &str) -> DebugReadFileResult {
    // Implement file reading logic
    DebugReadFileResult {
        contents_size: 0,
        contents: vec![],
    }
}

fn debug_platform_write_entire_file(_thread: &ThreadContext, filename: &str, data: &[u8]) -> bool {
    // Implement file writing logic
    true
}

fn megabytes(value: usize) -> usize {
    value * 1024 * 1024
}

fn gigabytes(value: usize) -> usize {
    value * 1024 * 1024 * 1024
}

// Implement key processing function
fn process_key_press(new_state: &mut GameButtonState, is_down: bool) {
    if new_state.ended_down != is_down {
        new_state.ended_down = is_down;
        new_state.half_transition_count += 1;
    }
}

fn main() {
    // Initialize SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create window and canvas
    let window = video_subsystem
        .window("Game Window", 960, 540)
        .position_centered()
        .resizable()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    // Create texture for rendering
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::ARGB8888, 960, 540)
        .unwrap();

    // Allocate game memory
    let mut game_memory = GameMemory {
        permanent_storage_size: megabytes(64),
        transient_storage_size: gigabytes(1),
        debug_platform_free_file_memory: Some(debug_platform_free_file_memory),
        debug_platform_read_entire_file: Some(debug_platform_read_entire_file),
        debug_platform_write_entire_file: Some(debug_platform_write_entire_file),
        // Initialize other fields as needed
        ..Default::default()
    };

    // Game input
    let mut game_input = GameInput {
        dt_for_frame: 1.0 / 30.0,
        controllers: [GameControllerInput {
            is_analog: false,
            stick_average_x: 0.0,
            stick_average_y: 0.0,
            buttons: [GameButtonState {
                half_transition_count: 0,
                ended_down: false,
            }; 12],
        }; 5],
    };

    // Offscreen buffer
    let mut offscreen_buffer = GameOffscreenBuffer {
        memory: vec![0u8; 960 * 540 * 4],
        width: 960,
        height: 540,
        pitch: 960 * 4,
        bytes_per_pixel: 4,
    };

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut running = true;

    while running {
        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => {
                    let controller = &mut game_input.controllers[0];
                    match scancode {
                        Scancode::W => {
                            process_key_press(&mut controller.buttons[MOVE_UP], true);
                        }
                        Scancode::A => {
                            process_key_press(&mut controller.buttons[MOVE_LEFT], true);
                        }
                        Scancode::S => {
                            process_key_press(&mut controller.buttons[MOVE_DOWN], true);
                        }
                        Scancode::D => {
                            process_key_press(&mut controller.buttons[MOVE_RIGHT], true);
                        }
                        _ => {}
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => {
                    let controller = &mut game_input.controllers[0];
                    match scancode {
                        Scancode::W => {
                            process_key_press(&mut controller.buttons[MOVE_UP], false);
                        }
                        Scancode::A => {
                            process_key_press(&mut controller.buttons[MOVE_LEFT], false);
                        }
                        Scancode::S => {
                            process_key_press(&mut controller.buttons[MOVE_DOWN], false);
                        }
                        Scancode::D => {
                            process_key_press(&mut controller.buttons[MOVE_RIGHT], false);
                        }
                        _ => {}
                    }
                }
                // Handle other events like mouse input here
                _ => {}
            }
        }

        // Update and render the game
        game_update_and_render(&mut game_memory, &game_input, &mut offscreen_buffer);

        // Update texture with the offscreen buffer
        texture
            .update(
                None,
                &offscreen_buffer.memory,
                offscreen_buffer.pitch as usize,
            )
            .unwrap();

        // Render to the screen
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
