// main.rs

extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::mem;

const PI32: f32 = 3.14159265359;

type bool32 = i32;

pub struct GameOffscreenBuffer<'a> {
    pub memory: &'a mut [u8],
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bytes_per_pixel: i32,
}

pub struct GameSoundOutputBuffer<'a> {
    samples_per_second: i32,
    sample_count: i32,
    samples: &'a mut [i16],
}

#[derive(Clone, Copy)]
pub struct GameButtonState {
    pub half_transition_count: i32,
    pub ended_down: bool,
}

#[derive(Clone, Copy)]
pub struct GameControllerInput {
    pub is_analog: bool,
    pub stick_average_x: f32,
    pub stick_average_y: f32,

    pub buttons: [GameButtonState; 12],
}

#[derive(Clone, Copy)]
pub struct GameInput {
    pub dt_for_frame: f32,
    pub controllers: [GameControllerInput; 5],
}

pub struct GameState {
    player_x: f32,
    player_y: f32,
}

fn game_output_sound(
    _game_state: &mut GameState,
    sound_buffer: &mut GameSoundOutputBuffer,
    _tone_hz: i32,
) {
    let tone_volume: i16 = 3000;
    let _wave_period = sound_buffer.samples_per_second / _tone_hz;

    let mut sample_out = 0;
    for _sample_index in 0..sound_buffer.sample_count {
        // Currently generating silence
        let sample_value: i16 = 0;

        sound_buffer.samples[sample_out] = sample_value;
        sound_buffer.samples[sample_out + 1] = sample_value;
        sample_out += 2;
    }
}

fn round_real32_to_int32(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

fn round_real32_to_uint32(value: f32) -> u32 {
    (value + 0.5).floor() as u32
}

fn truncate_real32_to_int32(value: f32) -> i32 {
    value as i32
}

fn draw_rectangle(
    buffer: &mut GameOffscreenBuffer,
    real_min_x: f32,
    real_min_y: f32,
    real_max_x: f32,
    real_max_y: f32,
    r: f32,
    g: f32,
    b: f32,
) {
    let mut min_x = round_real32_to_int32(real_min_x);
    let mut min_y = round_real32_to_int32(real_min_y);
    let mut max_x = round_real32_to_int32(real_max_x);
    let mut max_y = round_real32_to_int32(real_max_y);

    if min_x < 0 {
        min_x = 0;
    }
    if min_y < 0 {
        min_y = 0;
    }
    if max_x > buffer.width {
        max_x = buffer.width;
    }
    if max_y > buffer.height {
        max_y = buffer.height;
    }

    let color = ((round_real32_to_uint32(r * 255.0) << 16)
        | (round_real32_to_uint32(g * 255.0) << 8)
        | (round_real32_to_uint32(b * 255.0) << 0)) as u32;

    let mut row = min_y * buffer.pitch;
    for _y in min_y..max_y {
        let mut pixel_index = (row + min_x * buffer.bytes_per_pixel) as usize;
        for _x in min_x..max_x {
            buffer.memory[pixel_index..pixel_index + 4].copy_from_slice(&color.to_le_bytes());
            pixel_index += buffer.bytes_per_pixel as usize;
        }
        row += buffer.pitch;
    }
}

struct TileMap {
    count_x: i32,
    count_y: i32,
    upper_left_x: f32,
    upper_left_y: f32,
    tile_width: f32,
    tile_height: f32,
    tiles: Vec<u32>,
}

struct World {
    tile_map_count_x: i32,
    tile_map_count_y: i32,
    tile_maps: Vec<TileMap>,
}

fn get_tile_map<'a>(world: &'a World, tile_map_x: i32, tile_map_y: i32) -> Option<&'a TileMap> {
    if tile_map_x >= 0
        && tile_map_x < world.tile_map_count_x
        && tile_map_y >= 0
        && tile_map_y < world.tile_map_count_y
    {
        let index = (tile_map_y * world.tile_map_count_x + tile_map_x) as usize;
        Some(&world.tile_maps[index])
    } else {
        None
    }
}

fn get_tile_value_unchecked(tile_map: &TileMap, tile_x: i32, tile_y: i32) -> u32 {
    let index = (tile_y * tile_map.count_x + tile_x) as usize;
    tile_map.tiles[index]
}

fn is_tile_map_point_empty(tile_map: &TileMap, test_x: f32, test_y: f32) -> bool {
    let player_tile_x =
        truncate_real32_to_int32((test_x - tile_map.upper_left_x) / tile_map.tile_width);
    let player_tile_y =
        truncate_real32_to_int32((test_y - tile_map.upper_left_y) / tile_map.tile_height);

    if player_tile_x >= 0
        && player_tile_x < tile_map.count_x
        && player_tile_y >= 0
        && player_tile_y < tile_map.count_y
    {
        let tile_map_value = get_tile_value_unchecked(tile_map, player_tile_x, player_tile_y);
        tile_map_value == 0
    } else {
        false
    }
}

pub fn game_update_and_render(
    memory: &mut [u8],
    input: &GameInput,
    buffer: &mut GameOffscreenBuffer,
) {
    const TILE_MAP_COUNT_X: i32 = 17;
    const TILE_MAP_COUNT_Y: i32 = 9;

    // Define tile maps
    let tiles00 = [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    // Initialize game state from memory
    let game_state_ptr = memory.as_mut_ptr() as *mut GameState;
    let game_state = unsafe { &mut *game_state_ptr };

    if game_state.player_x == 0.0 && game_state.player_y == 0.0 {
        game_state.player_x = 150.0;
        game_state.player_y = 150.0;
    }

    let tile_map = TileMap {
        count_x: TILE_MAP_COUNT_X,
        count_y: TILE_MAP_COUNT_Y,
        upper_left_x: -30.0,
        upper_left_y: 0.0,
        tile_width: 60.0,
        tile_height: 60.0,
        tiles: tiles00.concat(),
    };

    let player_width = 0.75 * tile_map.tile_width;
    let player_height = tile_map.tile_height;

    // Process input
    for controller in input.controllers.iter() {
        if controller.is_analog {
            // Handle analog input
        } else {
            // Digital movement
            let mut dplayer_x = 0.0;
            let mut dplayer_y = 0.0;

            if controller.buttons[0].ended_down {
                dplayer_y = -1.0;
            }
            if controller.buttons[1].ended_down {
                dplayer_y = 1.0;
            }
            if controller.buttons[2].ended_down {
                dplayer_x = -1.0;
            }
            if controller.buttons[3].ended_down {
                dplayer_x = 1.0;
            }

            dplayer_x *= 64.0;
            dplayer_y *= 64.0;

            let new_player_x = game_state.player_x + input.dt_for_frame * dplayer_x;
            let new_player_y = game_state.player_y + input.dt_for_frame * dplayer_y;

            if is_tile_map_point_empty(&tile_map, new_player_x - 0.5 * player_width, new_player_y)
                && is_tile_map_point_empty(
                    &tile_map,
                    new_player_x + 0.5 * player_width,
                    new_player_y,
                )
                && is_tile_map_point_empty(&tile_map, new_player_x, new_player_y)
            {
                game_state.player_x = new_player_x;
                game_state.player_y = new_player_y;
            }
        }
    }

    // Render background
    draw_rectangle(
        buffer,
        0.0,
        0.0,
        buffer.width as f32,
        buffer.height as f32,
        1.0,
        0.0,
        0.1,
    );

    // Render tiles
    for row in 0..tile_map.count_y {
        for column in 0..tile_map.count_x {
            let tile_id = get_tile_value_unchecked(&tile_map, column, row);
            let gray = if tile_id == 1 { 1.0 } else { 0.5 };

            let min_x = tile_map.upper_left_x + (column as f32) * tile_map.tile_width;
            let min_y = tile_map.upper_left_y + (row as f32) * tile_map.tile_height;
            let max_x = min_x + tile_map.tile_width;
            let max_y = min_y + tile_map.tile_height;
            draw_rectangle(buffer, min_x, min_y, max_x, max_y, gray, gray, gray);
        }
    }

    // Render player
    let player_r = 1.0;
    let player_g = 1.0;
    let player_b = 0.0;
    let player_left = game_state.player_x - 0.5 * player_width;
    let player_top = game_state.player_y - player_height;
    draw_rectangle(
        buffer,
        player_left,
        player_top,
        player_left + player_width,
        player_top + player_height,
        player_r,
        player_g,
        player_b,
    );
}

fn game_get_sound_samples(memory: &mut [u8], sound_buffer: &mut GameSoundOutputBuffer) {
    let game_state_ptr = memory.as_mut_ptr() as *mut GameState;
    let game_state = unsafe { &mut *game_state_ptr };

    game_output_sound(game_state, sound_buffer, 400);
}

// main function and SDL setup would go here
