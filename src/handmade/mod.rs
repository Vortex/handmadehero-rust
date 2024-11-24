// main.rs

extern crate sdl2;

use std::cmp::{max, min};
use std::f32::consts::PI;
use std::mem;

type bool32 = i32;

const TILE_MAP_COUNT_X: i32 = 256;
const TILE_MAP_COUNT_Y: i32 = 256;

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

struct TileChunkPosition {
    tile_chunk_x: u32,
    tile_chunk_y: u32,

    rel_tile_x: u32,
    rel_tile_y: u32,
}

#[derive(Clone, Copy, Debug)]
struct WorldPosition {
    /* TODO:

       Take the tile map x and y
       and the tile x and y
       and pack them into single 32-bit value values for x and y
       where there is some low bits for the tile index
       and the high bits are the tile "page"

       (NOTE: we can eliminate the need for floor)
    */
    abs_tile_x: u32,
    abs_tile_y: u32,

    /* TODO:

      Should these be from the center of a tile?
      Rename to offset X and Y
    */
    tile_rel_x: f32,
    tile_rel_y: f32,
}

#[derive(Debug)]
struct TileChunk {
    tiles: Vec<u32>,
}

#[derive(Debug)]
struct World {
    chunk_shift: u32,
    chunk_mask: u32,
    chunk_dim: u32,

    tile_side_in_meters: f32,
    tile_side_in_pixels: i32,
    meters_to_pixels: f32,

    // TODO: Beginner's sparseness
    tile_chunk_count_x: i32,
    tile_chunk_count_y: i32,

    tile_chunks: Vec<TileChunk>,
}

pub struct GameState {
    pub player_p: WorldPosition,
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

fn get_tile_chunk<'a>(
    world: &'a World,
    tile_chunk_x: i32,
    tile_chunk_y: i32,
) -> Option<&'a TileChunk> {
    if tile_chunk_x >= 0
        && tile_chunk_x < world.tile_chunk_count_x
        && tile_chunk_y >= 0
        && tile_chunk_y < world.tile_chunk_count_y
    {
        let index = (tile_chunk_y * world.tile_chunk_count_x + tile_chunk_x) as usize;
        world.tile_chunks.get(index)
    } else {
        None
    }
}

fn get_tile_value_unchecked(
    world: &World,
    tile_chunk: &TileChunk,
    tile_x: u32,
    tile_y: u32,
) -> u32 {
    // Assertions (optional in Rust)
    assert!(tile_x < world.chunk_dim);
    assert!(tile_y < world.chunk_dim);

    let tile_chunk_value = tile_chunk.tiles[(tile_y * world.chunk_dim + tile_x) as usize];

    tile_chunk_value
}

fn get_tile_chunk_value(
    world: &World,
    tile_chunk: Option<&TileChunk>,
    test_tile_x: u32,
    test_tile_y: u32,
) -> u32 {
    let mut tile_chunk_value: u32 = 0;

    if let Some(tile_chunk) = tile_chunk {
        tile_chunk_value = get_tile_value_unchecked(world, tile_chunk, test_tile_x, test_tile_y);
    }

    tile_chunk_value
}

fn recanonicalize_coord(world: &World, tile: &mut u32, tile_rel: &mut f32) {
    let offset: i32 = floor_real32_to_int32(*tile_rel / world.tile_side_in_meters);

    if offset != 0 {
        if offset < 0 {
            *tile = tile.wrapping_sub((-offset) as u32);
        } else {
            *tile = tile.wrapping_add(offset as u32);
        }
        *tile_rel -= offset as f32 * world.tile_side_in_meters;
    }

    assert!(*tile_rel >= 0.0);
    assert!(*tile_rel <= world.tile_side_in_meters);
}

fn recanonicalize_position(world: &World, pos: WorldPosition) -> WorldPosition {
    let mut result = pos;

    recanonicalize_coord(world, &mut result.abs_tile_x, &mut result.tile_rel_x);
    recanonicalize_coord(world, &mut result.abs_tile_y, &mut result.tile_rel_y);

    result
}

fn get_chunk_position_for(world: &World, abs_tile_x: u32, abs_tile_y: u32) -> TileChunkPosition {
    let result = TileChunkPosition {
        tile_chunk_x: abs_tile_x >> world.chunk_shift,
        tile_chunk_y: abs_tile_y >> world.chunk_shift,

        rel_tile_x: abs_tile_x & world.chunk_mask,
        rel_tile_y: abs_tile_y & world.chunk_mask,
    };

    result
}

fn get_tile_value(world: &World, abs_tile_x: u32, abs_tile_y: u32) -> u32 {
    let mut empty = false;

    let chunk_pos = get_chunk_position_for(world, abs_tile_x, abs_tile_y);
    // let tile_map = get_tile_map(world, can_pos.tile_map_x, can_pos.tile_map_y);
    let tile_map = get_tile_chunk(
        world,
        chunk_pos.tile_chunk_x as i32,
        chunk_pos.tile_chunk_y as i32,
    );
    let tile_chunk_value =
        get_tile_chunk_value(world, tile_map, chunk_pos.rel_tile_x, chunk_pos.rel_tile_y);

    return tile_chunk_value;
}

fn is_world_point_empty(world: &World, pos: WorldPosition) -> bool {
    let tile_value = get_tile_value(world, pos.abs_tile_x, pos.abs_tile_y);
    return tile_value == 0;
}

fn create_tilemap() -> [[u32; TILE_MAP_COUNT_X as usize]; TILE_MAP_COUNT_Y as usize] {
    let mut tiles = [[0; TILE_MAP_COUNT_X as usize]; TILE_MAP_COUNT_Y as usize];

    // Define the visible part of the map
    let visible_map = [
        [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1,
        ],
        [
            1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1,
            1, 1, 1, 1, 1,
        ],
        [
            1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1,
            1, 1, 1, 1, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1,
        ],
        [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1,
        ],
    ];

    // Copy the visible map data into our zeroed array
    for (y, row) in visible_map.iter().enumerate() {
        tiles[y][..row.len()].copy_from_slice(row);
    }

    tiles
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

    // Define the tile maps
    let temp_tiles = create_tilemap();

    const TILE_SIDE_IN_PIXELS: i32 = 60;
    const TILE_SIDE_IN_METERS: f32 = 1.4;
    let tile_chunk = TileChunk {
        tiles: temp_tiles.concat(),
    };

    let world = World {
        chunk_shift: 8,
        chunk_mask: (1 << 8) - 1,
        chunk_dim: 256,

        tile_chunk_count_x: 1,
        tile_chunk_count_y: 1,

        // TODO: Begin using tile side in meters
        tile_side_in_meters: TILE_SIDE_IN_METERS,
        tile_side_in_pixels: TILE_SIDE_IN_PIXELS,
        meters_to_pixels: TILE_SIDE_IN_PIXELS as f32 / TILE_SIDE_IN_METERS,

        tile_chunks: vec![tile_chunk],
    };

    let player_height: f32 = 1.4;
    let player_width = 0.75 * player_height;

    let _lower_left_x = -world.tile_side_in_pixels as f32 / 2.0;
    let _lower_left_y = -buffer.height;

    // Initialize game state from memory
    let game_state_ptr = memory.permanent_storage.as_mut_ptr() as *mut GameState;
    let game_state = unsafe { &mut *game_state_ptr };

    if !memory.is_initialized {
        game_state.player_p.abs_tile_x = 3;
        game_state.player_p.abs_tile_y = 3;
        game_state.player_p.tile_rel_x = 5.0;
        game_state.player_p.tile_rel_y = 5.0;

        memory.is_initialized = true;
    }

    for controller in input.controllers.iter() {
        if controller.is_analog {
            // Handle analog input
        } else {
            // Digital movement
            let mut dplayer_x = 0.0;
            let mut dplayer_y = 0.0;

            if controller.buttons[0].ended_down {
                dplayer_y = 1.0;
            }
            if controller.buttons[1].ended_down {
                dplayer_y = -1.0;
            }
            if controller.buttons[2].ended_down {
                dplayer_x = -1.0;
            }
            if controller.buttons[3].ended_down {
                dplayer_x = 1.0;
            }

            dplayer_x *= 2.0;
            dplayer_y *= 2.0;

            let mut new_player_p: WorldPosition = game_state.player_p;
            new_player_p.tile_rel_x += input.dt_for_frame * dplayer_x;
            new_player_p.tile_rel_y += input.dt_for_frame * dplayer_y;
            new_player_p = recanonicalize_position(&world, new_player_p);

            let mut player_left = new_player_p;
            player_left.tile_rel_x -= 0.5 * player_width;
            player_left = recanonicalize_position(&world, player_left);

            let mut player_right = new_player_p;
            player_right.tile_rel_x += 0.5 * player_width;
            player_right = recanonicalize_position(&world, player_right);

            if is_world_point_empty(&world, new_player_p)
                && is_world_point_empty(&world, player_left)
                && is_world_point_empty(&world, player_right)
            {
                game_state.player_p = new_player_p;
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
    let center_x = 0.5 * buffer.width as f32;
    let center_y = 0.5 * buffer.height as f32;

    for rel_row in -10..10 {
        for rel_column in -20..20 {
            let column = game_state.player_p.abs_tile_x as i32 + rel_column;
            let row = game_state.player_p.abs_tile_y as i32 + rel_row;
            let tile_id = get_tile_value(&world, column as u32, row as u32);
            let mut gray: f32 = 0.5;

            if tile_id == 1 {
                gray = 1.0;
            }

            if column as u32 == game_state.player_p.abs_tile_x
                && row as u32 == game_state.player_p.abs_tile_y
            {
                gray = 0.0;
            }

            let min_x = center_x + (rel_column * world.tile_side_in_pixels) as f32;
            let min_y = center_y - (rel_row * world.tile_side_in_pixels) as f32;

            let max_x = min_x + world.tile_side_in_pixels as f32;
            let max_y = min_y - world.tile_side_in_pixels as f32;

            draw_rectangle(buffer, min_x, max_y, max_x, min_y, gray, gray, gray);
        }
    }

    // Render player
    let player_r = 1.0;
    let player_g = 1.0;
    let player_b = 0.0;

    let player_left = center_x + world.meters_to_pixels * game_state.player_p.tile_rel_x
        - 0.5 * player_width * world.meters_to_pixels;

    let player_top = center_y
        - world.meters_to_pixels * game_state.player_p.tile_rel_y
        - world.meters_to_pixels * player_height;

    draw_rectangle(
        buffer,
        player_left,
        player_top,
        player_left + world.meters_to_pixels * player_width,
        player_top + world.meters_to_pixels * player_height,
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
