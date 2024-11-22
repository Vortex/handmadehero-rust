// main.rs

extern crate sdl2;

use std::cmp::{max, min};
use std::f32::consts::PI;
use std::mem;

type bool32 = i32;

// Thread context
pub struct ThreadContext {
    placeholder: i32,
}

// Debug read file result
pub struct DebugReadFileResult {
    pub contents_size: u32,
    pub contents: Vec<u8>,
}
#[derive(Clone)]
pub struct GameOffscreenBuffer {
    pub memory: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bytes_per_pixel: i32,
}

struct GameSoundOutputBuffer<'a> {
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
    // pub is_connected: bool,
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

// Game memory
#[derive(Default)]
pub struct GameMemory {
    pub is_initialized: bool,

    pub permanent_storage_size: usize,
    pub permanent_storage: Vec<u8>,

    pub transient_storage_size: usize,
    pub transient_storage: Vec<u8>,
    // Debug functions (optional)
    pub debug_platform_free_file_memory: Option<fn(&ThreadContext, &mut [u8])>,
    pub debug_platform_read_entire_file: Option<fn(&ThreadContext, &str) -> DebugReadFileResult>,
    pub debug_platform_write_entire_file: Option<fn(&ThreadContext, &str, &[u8]) -> bool>,
}

pub struct GameState {
    player_tile_map_x: i32,
    player_tile_map_y: i32,
    player_x: f32,
    player_y: f32,
    t_sine: f32,
}

#[derive(Debug)]
struct TileMap {
    tiles: Vec<u32>,
}

#[derive(Debug)]
struct World {
    tile_map_count_x: i32,
    tile_map_count_y: i32,
    count_x: i32,
    count_y: i32,
    upper_left_x: f32,
    upper_left_y: f32,
    tile_width: f32,
    tile_height: f32,
    tile_maps: Vec<TileMap>,
}

#[derive(Clone, Copy, Debug)]
struct RawPosition {
    tile_map_x: i32,
    tile_map_y: i32,
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct CanonicalPosition {
    tile_map_x: i32,
    tile_map_y: i32,
    tile_x: i32,
    tile_y: i32,
    tile_rel_x: f32,
    tile_rel_y: f32,
}

fn game_output_sound(
    _game_state: &mut GameState,
    sound_buffer: &mut GameSoundOutputBuffer,
    _tone_hz: i32,
) {
    let tone_volume: i16 = 3000;
    let _wave_period = sound_buffer.samples_per_second / _tone_hz;

    let samples = &mut sound_buffer.samples;
    let sample_count = sound_buffer.sample_count as usize;

    for sample_index in 0..sample_count {
        // TODO: Draw this out for people
        // The original code has sine wave generation commented out
        let sample_value: i16 = 0;

        let index = sample_index * 2;
        if index + 1 < samples.len() {
            samples[index] = sample_value;
            samples[index + 1] = sample_value;
        }
    }
}

fn round_real32_to_int32(value: f32) -> i32 {
    (value + 0.5) as i32
}

fn round_real32_to_uint32(value: f32) -> u32 {
    (value + 0.5) as u32
}

fn floor_real32_to_int32(value: f32) -> i32 {
    value.floor() as i32
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

    min_x = max(0, min_x);
    min_y = max(0, min_y);
    max_x = min(buffer.width, max_x);
    max_y = min(buffer.height, max_y);

    let color = ((round_real32_to_uint32(r * 255.0) << 16)
        | (round_real32_to_uint32(g * 255.0) << 8)
        | (round_real32_to_uint32(b * 255.0) << 0)) as u32;

    let bytes_per_pixel = buffer.bytes_per_pixel as usize;
    let pitch = buffer.pitch as usize;

    for y in min_y..max_y {
        let row_start = (y as usize) * pitch + (min_x as usize) * bytes_per_pixel;
        for x in min_x..max_x {
            let pixel_index = row_start + (x as usize - min_x as usize) * bytes_per_pixel;
            if pixel_index + 3 < buffer.memory.len() {
                buffer.memory[pixel_index..pixel_index + 4].copy_from_slice(&color.to_le_bytes());
            }
        }
    }
}

fn get_tile_map<'a>(world: &'a World, tile_map_x: i32, tile_map_y: i32) -> Option<&'a TileMap> {
    if tile_map_x >= 0
        && tile_map_x < world.tile_map_count_x
        && tile_map_y >= 0
        && tile_map_y < world.tile_map_count_y
    {
        let index = (tile_map_y * world.tile_map_count_x + tile_map_x) as usize;
        world.tile_maps.get(index)
    } else {
        None
    }
}

fn get_tile_value_unchecked(world: &World, tile_map: &TileMap, x: i32, y: i32) -> u32 {
    // Assertions (optional in Rust)
    assert!(x >= 0 && x < world.count_x);
    assert!(y >= 0 && y < world.count_y);

    let index = (y * world.count_x + x) as usize;
    tile_map.tiles[index]
}

fn is_tile_map_point_empty(
    world: &World,
    tile_map: Option<&TileMap>,
    tile_x: i32,
    tile_y: i32,
) -> bool {
    let mut empty: bool = false;

    if let Some(tile_map) = tile_map {
        if tile_x >= 0 && tile_x < world.count_x && tile_y >= 0 && tile_y < world.count_y {
            let tile_value = get_tile_value_unchecked(world, tile_map, tile_x, tile_y);
            empty = tile_value == 0;
        }
    }

    return empty;
}

fn get_canonical_position(world: &World, pos: RawPosition) -> CanonicalPosition {
    let mut result = CanonicalPosition {
        tile_map_x: pos.tile_map_x,
        tile_map_y: pos.tile_map_y,
        tile_x: 0,
        tile_y: 0,
        tile_rel_x: 0.0,
        tile_rel_y: 0.0,
    };

    // Calculate X and Y relative to the world's upper-left corner
    let x = pos.x - world.upper_left_x;
    let y = pos.y - world.upper_left_y;

    // Calculate tile indices using floor division
    result.tile_x = floor_real32_to_int32(x / world.tile_width);
    result.tile_y = floor_real32_to_int32(y / world.tile_height);

    // Calculate the relative position within the tile
    result.tile_rel_x = x - (result.tile_x as f32) * world.tile_width;
    result.tile_rel_y = y - (result.tile_y as f32) * world.tile_height;

    // Assertions to ensure tile_rel_x and tile_rel_y are within valid ranges
    assert!(result.tile_rel_x >= 0.0);
    assert!(result.tile_rel_y >= 0.0);
    assert!(result.tile_rel_x < world.tile_width);
    assert!(result.tile_rel_y < world.tile_height);

    // Adjust tile_x and tile_map_x if tile_x is out of bounds
    if result.tile_x < 0 {
        result.tile_x = world.count_x + result.tile_x;
        result.tile_map_x -= 1;
    }

    if result.tile_x >= world.count_x {
        result.tile_x = result.tile_x - world.count_x;
        result.tile_map_x += 1;
    }

    // Adjust tile_y and tile_map_y if tile_y is out of bounds
    if result.tile_y < 0 {
        result.tile_y = world.count_y + result.tile_y;
        result.tile_map_y -= 1;
    }

    if result.tile_y >= world.count_y {
        result.tile_y = result.tile_y - world.count_y;
        result.tile_map_y += 1;
    }

    result
}

fn is_world_point_empty(world: &World, test_pos: RawPosition) -> bool {
    let mut empty = false;

    let can_pos = get_canonical_position(world, test_pos);
    let tile_map = get_tile_map(world, can_pos.tile_map_x, can_pos.tile_map_y);
    empty = is_tile_map_point_empty(world, tile_map, can_pos.tile_x, can_pos.tile_y);

    return empty;
}
pub fn game_update_and_render(
    memory: &mut GameMemory,
    input: &GameInput,
    buffer: &mut GameOffscreenBuffer,
) {
    if memory.permanent_storage.len() < mem::size_of::<GameState>() {
        memory
            .permanent_storage
            .resize(mem::size_of::<GameState>(), 0);
    }

    const TILE_MAP_COUNT_X: i32 = 17;
    const TILE_MAP_COUNT_Y: i32 = 9;

    // Define the tile maps
    let tiles00: [[u32; TILE_MAP_COUNT_X as usize]; TILE_MAP_COUNT_Y as usize] = [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let tiles01: [[u32; TILE_MAP_COUNT_X as usize]; TILE_MAP_COUNT_Y as usize] = [
        [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let tiles10: [[u32; TILE_MAP_COUNT_X as usize]; TILE_MAP_COUNT_Y as usize] = [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let tiles11: [[u32; TILE_MAP_COUNT_X as usize]; TILE_MAP_COUNT_Y as usize] = [
        [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];
    // Initialize tile maps
    let tile_map00 = TileMap {
        tiles: tiles00.concat(),
    };
    let tile_map01 = TileMap {
        tiles: tiles01.concat(),
    };
    let tile_map10 = TileMap {
        tiles: tiles10.concat(),
    };
    let tile_map11 = TileMap {
        tiles: tiles11.concat(),
    };

    let tile_maps = vec![tile_map00, tile_map10, tile_map01, tile_map11];

    let world = World {
        tile_map_count_x: 2,
        tile_map_count_y: 2,
        count_x: TILE_MAP_COUNT_X,
        count_y: TILE_MAP_COUNT_Y,
        upper_left_x: -30.0,
        upper_left_y: 0.0,
        tile_width: 60.0,
        tile_height: 60.0,
        tile_maps,
    };

    let player_width = 0.75 * world.tile_width;
    let player_height = world.tile_height;

    // Initialize game state from memory
    let game_state_ptr = memory.permanent_storage.as_mut_ptr() as *mut GameState;
    let game_state = unsafe { &mut *game_state_ptr };

    if !memory.is_initialized {
        game_state.player_tile_map_x = 0;
        game_state.player_tile_map_y = 0;
        game_state.player_x = 150.0;
        game_state.player_y = 150.0;
        game_state.t_sine = 0.0;

        memory.is_initialized = true;
    }

    // Determine the current tile map based on the player's position
    let current_tile_map_x = game_state.player_tile_map_x;
    let current_tile_map_y = game_state.player_tile_map_y;

    // Get the current tile map
    let tile_map =
        get_tile_map(&world, current_tile_map_x, current_tile_map_y).expect("Tile map not found");

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

            dplayer_x *= 128.0;
            dplayer_y *= 128.0;

            let new_player_x = game_state.player_x + input.dt_for_frame * dplayer_x;
            let new_player_y = game_state.player_y + input.dt_for_frame * dplayer_y;

            let player_pos = RawPosition {
                tile_map_x: game_state.player_tile_map_x,
                tile_map_y: game_state.player_tile_map_y,
                x: new_player_x,
                y: new_player_y,
            };
            let mut player_left = player_pos.clone();
            player_left.x -= 0.5 * player_width;
            let mut player_right = player_pos.clone();
            player_right.x += 0.5 * player_width;

            if is_world_point_empty(&world, player_pos)
                && is_world_point_empty(&world, player_left)
                && is_world_point_empty(&world, player_right)
            {
                let can_pos = get_canonical_position(&world, player_pos);

                game_state.player_tile_map_x = can_pos.tile_map_x;
                game_state.player_tile_map_y = can_pos.tile_map_y;
                game_state.player_x = world.upper_left_x
                    + world.tile_width * can_pos.tile_x as f32
                    + can_pos.tile_rel_x;
                game_state.player_y = world.upper_left_y
                    + world.tile_height * can_pos.tile_y as f32
                    + can_pos.tile_rel_y;
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
    for row in 0..world.count_y {
        for column in 0..world.count_x {
            let tile_id = get_tile_value_unchecked(&world, tile_map, column, row);
            let gray = if tile_id == 1 { 1.0 } else { 0.5 };

            let min_x = world.upper_left_x + (column as f32) * world.tile_width;
            let min_y = world.upper_left_y + (row as f32) * world.tile_height;
            let max_x = min_x + world.tile_width;
            let max_y = min_y + world.tile_height;
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

fn game_get_sound_samples(memory: &mut GameMemory, sound_buffer: &mut GameSoundOutputBuffer) {
    let game_state_ptr = memory.permanent_storage.as_mut_ptr() as *mut GameState;
    let game_state = unsafe { &mut *game_state_ptr };

    game_output_sound(game_state, sound_buffer, 400);
}
