#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, player::*, resources::*, storage::*,
    terrain_assets::*, ui::*, world::*,
};

#[allow(dead_code)]
pub(crate) enum HudIconKind {
    Empty,
    Heart,
    Stone,
    CopperOre,
    Coal,
    IronOre,
    IronPlate,
    CopperPlate,
    Furnace,
    Chest,
    Inserter,
    Anvil,
}

pub(crate) fn build_hud_icon_image(kind: HudIconKind) -> Image {
    let size = 16usize;
    let mut pixels = vec![0u8; size * size * 4];

    match kind {
        HudIconKind::Empty => {}
        HudIconKind::Heart => {
            let mask = [
                "0000000000000000",
                "0001100001100000",
                "0011110011110000",
                "0111111111111000",
                "0111111111111000",
                "0011111111110000",
                "0001111111100000",
                "0000111111000000",
                "0000011110000000",
                "0000001100000000",
                "0000000000000000",
                "0000000000000000",
                "0000000000000000",
                "0000000000000000",
                "0000000000000000",
                "0000000000000000",
            ];
            paint_icon_mask(
                &mut pixels,
                size,
                &mask,
                [79, 231, 105, 255],
                [21, 122, 42, 255],
                [42, 184, 70, 255],
            );
        }
        HudIconKind::Stone => {
            let mask = rock_mask();
            paint_icon_mask(
                &mut pixels,
                size,
                &mask,
                [142, 148, 151, 255],
                [74, 80, 82, 255],
                [177, 184, 186, 255],
            );
        }
        HudIconKind::CopperOre => {
            let mask = rock_mask();
            paint_icon_mask(
                &mut pixels,
                size,
                &mask,
                [191, 116, 68, 255],
                [104, 61, 38, 255],
                [228, 151, 94, 255],
            );
        }
        HudIconKind::Coal => {
            let mask = rock_mask();
            paint_icon_mask(
                &mut pixels,
                size,
                &mask,
                [42, 44, 48, 255],
                [12, 13, 15, 255],
                [70, 73, 80, 255],
            );
        }
        HudIconKind::IronOre => {
            let mask = rock_mask();
            paint_icon_mask(
                &mut pixels,
                size,
                &mask,
                [111, 158, 186, 255],
                [49, 76, 94, 255],
                [165, 206, 226, 255],
            );
            set_icon_pixel(&mut pixels, size, 7, 4, [220, 238, 246, 255]);
            set_icon_pixel(&mut pixels, size, 10, 7, [205, 229, 241, 255]);
        }
        HudIconKind::IronPlate => {
            paint_plate_icon(
                &mut pixels,
                size,
                [122, 156, 172, 255],
                [58, 78, 88, 255],
                [188, 214, 226, 255],
            );
        }
        HudIconKind::CopperPlate => {
            paint_plate_icon(
                &mut pixels,
                size,
                [196, 113, 66, 255],
                [105, 57, 35, 255],
                [236, 158, 98, 255],
            );
        }
        HudIconKind::Furnace => {
            fill_icon_rect(&mut pixels, size, 3, 3, 12, 13, [68, 64, 58, 255]);
            fill_icon_rect(&mut pixels, size, 4, 4, 11, 12, [118, 112, 101, 255]);
            fill_icon_rect(&mut pixels, size, 5, 7, 10, 11, [31, 27, 24, 255]);
            fill_icon_rect(&mut pixels, size, 6, 8, 9, 11, [232, 104, 45, 255]);
            fill_icon_rect(&mut pixels, size, 7, 8, 8, 10, [255, 188, 62, 255]);
            fill_icon_rect(&mut pixels, size, 4, 2, 11, 3, [47, 44, 40, 255]);
        }
        HudIconKind::Chest => {
            fill_icon_rect(&mut pixels, size, 3, 5, 12, 12, [111, 72, 38, 255]);
            fill_icon_rect(&mut pixels, size, 4, 4, 11, 6, [155, 102, 53, 255]);
            fill_icon_rect(&mut pixels, size, 3, 7, 12, 8, [81, 52, 29, 255]);
            fill_icon_rect(&mut pixels, size, 7, 7, 8, 9, [218, 178, 83, 255]);
            fill_icon_rect(&mut pixels, size, 4, 10, 11, 12, [128, 81, 42, 255]);
        }
        HudIconKind::Inserter => {
            fill_icon_rect(&mut pixels, size, 3, 7, 12, 9, [49, 51, 48, 255]);
            fill_icon_rect(&mut pixels, size, 4, 6, 7, 10, [205, 150, 49, 255]);
            fill_icon_rect(&mut pixels, size, 8, 5, 10, 11, [226, 174, 62, 255]);
            fill_icon_rect(&mut pixels, size, 11, 6, 12, 10, [78, 82, 76, 255]);
            fill_icon_rect(&mut pixels, size, 6, 4, 9, 5, [242, 204, 92, 255]);
            fill_icon_rect(&mut pixels, size, 6, 11, 9, 12, [144, 96, 32, 255]);
            set_icon_pixel(&mut pixels, size, 12, 5, [235, 224, 151, 255]);
            set_icon_pixel(&mut pixels, size, 13, 8, [235, 224, 151, 255]);
        }
        HudIconKind::Anvil => {
            fill_icon_rect(&mut pixels, size, 4, 3, 12, 5, [50, 53, 56, 255]);
            fill_icon_rect(&mut pixels, size, 3, 4, 13, 6, [137, 141, 143, 255]);
            fill_icon_rect(&mut pixels, size, 1, 5, 5, 7, [137, 141, 143, 255]);
            fill_icon_rect(&mut pixels, size, 6, 7, 10, 10, [114, 118, 121, 255]);
            fill_icon_rect(&mut pixels, size, 4, 11, 12, 13, [50, 53, 56, 255]);
            fill_icon_rect(&mut pixels, size, 5, 11, 11, 12, [146, 151, 153, 255]);
            set_icon_pixel(&mut pixels, size, 12, 3, [210, 213, 214, 255]);
            set_icon_pixel(&mut pixels, size, 13, 4, [210, 213, 214, 255]);
        }
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub(crate) fn rock_mask() -> [&'static str; 16] {
    [
        "0000000000000000",
        "0000001110000000",
        "0000111111100000",
        "0001111111110000",
        "0011111111111000",
        "0011111111111000",
        "0111111111110000",
        "0011111111110000",
        "0001111111100000",
        "0000111111000000",
        "0000011100000000",
        "0000000000000000",
        "0000000000000000",
        "0000000000000000",
        "0000000000000000",
        "0000000000000000",
    ]
}

pub(crate) fn paint_plate_icon(
    pixels: &mut [u8],
    size: usize,
    fill: [u8; 4],
    edge: [u8; 4],
    highlight: [u8; 4],
) {
    fill_icon_rect(pixels, size, 3, 4, 12, 11, edge);
    fill_icon_rect(pixels, size, 4, 3, 11, 10, fill);
    fill_icon_rect(pixels, size, 5, 4, 10, 5, highlight);
    fill_icon_rect(pixels, size, 4, 10, 11, 11, edge);
}

pub(crate) fn paint_icon_mask(
    pixels: &mut [u8],
    size: usize,
    mask: &[&str; 16],
    fill: [u8; 4],
    edge: [u8; 4],
    highlight: [u8; 4],
) {
    let is_filled = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
            return false;
        }
        mask.get(y as usize)
            .and_then(|row| row.as_bytes().get(x as usize))
            .copied()
            == Some(b'1')
    };

    for y in 0..size as i32 {
        for x in 0..size as i32 {
            if !is_filled(x, y) {
                continue;
            }
            let is_edge = !is_filled(x - 1, y)
                || !is_filled(x + 1, y)
                || !is_filled(x, y - 1)
                || !is_filled(x, y + 1);
            let color = if is_edge {
                edge
            } else if (x + y * 2) % 7 == 0 {
                highlight
            } else {
                fill
            };
            set_icon_pixel(pixels, size, x, y, color);
        }
    }
}

pub(crate) fn set_icon_pixel(pixels: &mut [u8], size: usize, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
        return;
    }
    let index = (y as usize * size + x as usize) * 4;
    pixels[index..index + 4].copy_from_slice(&color);
}

pub(crate) fn fill_icon_rect(
    pixels: &mut [u8],
    size: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            set_icon_pixel(pixels, size, x, y, color);
        }
    }
}

pub(crate) fn build_player_image() -> Image {
    let size = 16usize;
    let mut filled = vec![false; size * size];

    let mut fill_rect = |x0: usize, y0: usize, x1: usize, y1: usize| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                filled[y * size + x] = true;
            }
        }
    };

    fill_rect(5, 2, 10, 5); // head
    fill_rect(4, 6, 11, 11); // body
    fill_rect(4, 12, 6, 13); // left foot
    fill_rect(9, 12, 11, 13); // right foot

    let is_filled = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
            return false;
        }
        filled[y as usize * size + x as usize]
    };

    let is_foot =
        |x: usize, y: usize| (y >= 12 && y <= 13) && ((x >= 4 && x <= 6) || (x >= 9 && x <= 11));

    let outline_color = [28, 22, 18, 255];
    let body_color = [226, 205, 124, 255];
    let foot_color = [190, 168, 96, 255];
    let mut pixels = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            if !filled[y * size + x] {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let outline = !is_filled(x as i32 - 1, y as i32)
                || !is_filled(x as i32 + 1, y as i32)
                || !is_filled(x as i32, y as i32 - 1)
                || !is_filled(x as i32, y as i32 + 1);
            let color = if outline {
                outline_color
            } else if is_foot(x, y) {
                foot_color
            } else {
                body_color
            };
            pixels.extend_from_slice(&color);
        }
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub(crate) const TERRAIN_TILE_PIXELS: usize = 8;
pub(crate) const CHUNK_TEXTURE_EDGE: usize = (CHUNK_EDGE as usize + 2) * TERRAIN_TILE_PIXELS;

pub(crate) fn build_chunk_image(
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) -> Image {
    let pixels = chunk_pixels(data, config, world_seed, highlight);
    let mut image = Image::new_fill(
        Extent3d {
            width: CHUNK_TEXTURE_EDGE as u32,
            height: CHUNK_TEXTURE_EDGE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub(crate) fn chunk_sprite_rect() -> Rect {
    let pad = TERRAIN_TILE_PIXELS as f32;
    let edge = CHUNK_EDGE as f32 * TERRAIN_TILE_PIXELS as f32;
    Rect::from_corners(Vec2::new(pad, pad), Vec2::new(pad + edge, pad + edge))
}

pub(crate) fn refresh_chunk_texture(
    images: &mut Assets<Image>,
    handle: &Handle<Image>,
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) {
    if let Some(image) = images.get_mut(handle) {
        *image = build_chunk_image(data, config, world_seed, highlight);
    }
}

pub(crate) fn chunk_pixels(
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) -> Vec<u8> {
    let edge = CHUNK_EDGE as usize;
    let padded_tiles = edge + 2;
    let mut pixels = Vec::with_capacity(CHUNK_TEXTURE_EDGE * CHUNK_TEXTURE_EDGE * 4);

    for tile_oy in 0..padded_tiles {
        let base_ty = if tile_oy == 0 {
            0
        } else if tile_oy > edge {
            edge - 1
        } else {
            tile_oy - 1
        };
        let ty = edge - 1 - base_ty;
        let interior_y = tile_oy as i32 - 1;
        for sub_y in 0..TERRAIN_TILE_PIXELS {
            for tile_ox in 0..padded_tiles {
                let tx = if tile_ox == 0 {
                    0
                } else if tile_ox > edge {
                    edge - 1
                } else {
                    tile_ox - 1
                };
                let interior_x = tile_ox as i32 - 1;
                let tile = tile_at(data, tx as i32, ty as i32, world_seed);
                let gx = data.coord.cx * CHUNK_EDGE as i32 + tx as i32;
                let gy = data.coord.cy * CHUNK_EDGE as i32 + ty as i32;

                for sub_x in 0..TERRAIN_TILE_PIXELS {
                    let mut color = detailed_terrain_color(
                        data, tile, tx as i32, ty as i32, gx, gy, sub_x, sub_y, world_seed,
                    );

                    let resource = resource_at(data, tx as i32, ty as i32);
                    if resource.kind != RES_NONE && resource.amount > 0 {
                        color = apply_resource_overlay(
                            data,
                            color,
                            resource.kind,
                            tx as i32,
                            ty as i32,
                            gx,
                            gy,
                            sub_x,
                            sub_y,
                            world_seed,
                        );
                    }
                    let placed = placed_at(data, tx as i32, ty as i32);
                    if placed.kind != PLACED_NONE {
                        color = apply_placed_sprite(color, placed.kind, sub_x, sub_y);
                    }
                    if config.show_chunk_borders
                        && interior_x >= 0
                        && interior_y >= 0
                        && (interior_x == 0 || interior_y == 0)
                    {
                        color = darken_color(color);
                    }
                    if let Some((hx, hy)) = highlight {
                        if gx == hx && gy == hy {
                            color = blend_color(color, [220, 40, 40, 255], 0.8);
                        }
                    }
                    pixels.extend_from_slice(&color);
                }
            }
        }
    }
    pixels
}

fn detailed_terrain_color(
    data: &SimChunkData,
    tile: TileId,
    tx: i32,
    ty: i32,
    gx: i32,
    gy: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let mut color = sample_terrain_texture(tile, gx, gy, sub_x, sub_y, world_seed);
    let jitter = detail_jitter(gx, gy, sub_x, sub_y, world_seed);
    color = adjust_color(color, jitter);
    color = apply_tile_variant_tint(color, tile);

    if is_water(tile) {
        color = apply_water_edges(data, color, tx, ty, sub_x, sub_y, world_seed);
    } else {
        color = apply_land_edges(data, color, tile, tx, ty, sub_x, sub_y, world_seed);
    }

    color
}

fn sample_terrain_texture(
    tile: TileId,
    gx: i32,
    gy: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let texture = if is_water(tile) {
        &WATER_TEXTURE
    } else if is_dirt_tile(tile) {
        &DIRT_TEXTURE
    } else {
        &GRASS_TEXTURE
    };
    let world_px = gx
        .saturating_mul(TERRAIN_TILE_PIXELS as i32)
        .saturating_add(sub_x as i32);
    let world_py = gy
        .saturating_mul(TERRAIN_TILE_PIXELS as i32)
        .saturating_add((TERRAIN_TILE_PIXELS - 1 - sub_y) as i32);
    let seed_shift_x = ((world_seed >> 7) & 31) as i32;
    let seed_shift_y = ((world_seed >> 17) & 31) as i32;
    let x = (world_px + seed_shift_x).rem_euclid(TERRAIN_TEXTURE_EDGE as i32) as usize;
    let y = (world_py + seed_shift_y).rem_euclid(TERRAIN_TEXTURE_EDGE as i32) as usize;
    texture[y * TERRAIN_TEXTURE_EDGE + x]
}

fn apply_water_edges(
    data: &SimChunkData,
    mut color: [u8; 4],
    tx: i32,
    ty: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let land = |dx: i32, dy: i32| !is_water(tile_at(data, tx + dx, ty + dy, world_seed));
    let shallow = shallow_water_color();
    let bank = [91, 73, 45, 255];
    let edge = strongest_edge_weight(land, sub_x, sub_y, 4);
    if edge > 0.0 {
        color = blend_color(color, shallow, 0.38 * edge);
    }
    let bank_edge = strongest_edge_weight(land, sub_x, sub_y, 2);
    if bank_edge > 0.0 {
        color = blend_color(color, bank, 0.46 * bank_edge);
    }
    color
}

fn apply_land_edges(
    data: &SimChunkData,
    mut color: [u8; 4],
    tile: TileId,
    tx: i32,
    ty: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let water = |dx: i32, dy: i32| is_water(tile_at(data, tx + dx, ty + dy, world_seed));
    let water_edge = strongest_edge_weight(water, sub_x, sub_y, 3);
    if water_edge > 0.0 {
        color = blend_color(color, [83, 70, 44, 255], 0.55 * water_edge);
    }

    let neighbor_is_dirt =
        |dx: i32, dy: i32| is_dirt_tile(tile_at(data, tx + dx, ty + dy, world_seed));
    let neighbor_is_grass = |dx: i32, dy: i32| {
        let other = tile_at(data, tx + dx, ty + dy, world_seed);
        !is_water(other) && !is_dirt_tile(other)
    };

    if !is_dirt_tile(tile) {
        let dirt_edge = strongest_edge_weight(neighbor_is_dirt, sub_x, sub_y, 3);
        if dirt_edge > 0.0 {
            let dirt = sample_texture_at(&DIRT_TEXTURE, tx, ty, sub_x, sub_y, world_seed);
            color = blend_color(color, dirt, 0.36 * dirt_edge);
        }
    } else {
        let grass_edge = strongest_edge_weight(neighbor_is_grass, sub_x, sub_y, 2);
        if grass_edge > 0.0 {
            let grass = sample_texture_at(&GRASS_TEXTURE, tx, ty, sub_x, sub_y, world_seed);
            color = blend_color(color, grass, 0.22 * grass_edge);
        }
    }

    color
}

fn sample_texture_at(
    texture: &[[u8; 4]; TERRAIN_TEXTURE_LEN],
    tx: i32,
    ty: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let world_px = tx
        .saturating_mul(TERRAIN_TILE_PIXELS as i32)
        .saturating_add(sub_x as i32);
    let world_py = ty
        .saturating_mul(TERRAIN_TILE_PIXELS as i32)
        .saturating_add((TERRAIN_TILE_PIXELS - 1 - sub_y) as i32);
    let seed_shift_x = ((world_seed >> 7) & 31) as i32;
    let seed_shift_y = ((world_seed >> 17) & 31) as i32;
    let x = (world_px + seed_shift_x).rem_euclid(TERRAIN_TEXTURE_EDGE as i32) as usize;
    let y = (world_py + seed_shift_y).rem_euclid(TERRAIN_TEXTURE_EDGE as i32) as usize;
    texture[y * TERRAIN_TEXTURE_EDGE + x]
}

fn apply_resource_overlay(
    data: &SimChunkData,
    base: [u8; 4],
    kind: ResourceId,
    tx: i32,
    ty: i32,
    gx: i32,
    gy: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let noise = resource_noise(gx, gy, sub_x, sub_y, world_seed, kind as u64);
    let edge = strongest_edge_weight(
        |dx, dy| !resource_matches(data, kind, tx + dx, ty + dy, world_seed),
        sub_x,
        sub_y,
        3,
    );
    let edge_fade = 1.0 - edge * 0.68;

    let dx = sub_x as f32 - (TERRAIN_TILE_PIXELS as f32 - 1.0) * 0.5;
    let dy = sub_y as f32 - (TERRAIN_TILE_PIXELS as f32 - 1.0) * 0.5;
    let radial = 1.0 - ((dx * dx + dy * dy).sqrt() / 5.2).clamp(0.0, 1.0);

    if noise < resource_hole_threshold(kind) && radial < 0.82 {
        return base;
    }

    let mut overlay = if let Some(texture) = resource_texture(kind) {
        sample_resource_texture(texture, gx, gy, sub_x, sub_y, world_seed)
    } else {
        adjust_color(resource_color(kind), ((noise * 30.0).round() as i16) - 15)
    };

    let vein = resource_noise(
        gx,
        gy,
        sub_x,
        sub_y,
        world_seed.rotate_left(17),
        0x4f1b_bcdc_9d5a_4337 ^ kind as u64,
    );
    if vein > 0.88 {
        overlay = blend_color(overlay, resource_highlight(kind), 0.34);
    }

    let clump = 0.48 + noise * 0.52;
    let shape = 0.72 + radial * 0.24;
    let weight = (resource_overlay_weight(kind) * clump * shape * edge_fade).clamp(0.0, 0.86);
    blend_color(base, overlay, weight)
}

fn resource_texture(kind: ResourceId) -> Option<&'static [[u8; 4]; RESOURCE_TEXTURE_LEN]> {
    match kind {
        RES_COAL => Some(&COAL_RESOURCE_TEXTURE),
        RES_IRON => Some(&IRON_RESOURCE_TEXTURE),
        RES_COPPER => Some(&COPPER_RESOURCE_TEXTURE),
        _ => None,
    }
}

fn sample_resource_texture(
    texture: &[[u8; 4]; RESOURCE_TEXTURE_LEN],
    gx: i32,
    gy: i32,
    sub_x: usize,
    sub_y: usize,
    world_seed: u64,
) -> [u8; 4] {
    let world_px = gx
        .saturating_mul(TERRAIN_TILE_PIXELS as i32)
        .saturating_add(sub_x as i32);
    let world_py = gy
        .saturating_mul(TERRAIN_TILE_PIXELS as i32)
        .saturating_add((TERRAIN_TILE_PIXELS - 1 - sub_y) as i32);
    let seed_shift_x = ((world_seed >> 23) & 31) as i32;
    let seed_shift_y = ((world_seed >> 37) & 31) as i32;
    let x = (world_px + seed_shift_x).rem_euclid(RESOURCE_TEXTURE_EDGE as i32) as usize;
    let y = (world_py + seed_shift_y).rem_euclid(RESOURCE_TEXTURE_EDGE as i32) as usize;
    texture[y * RESOURCE_TEXTURE_EDGE + x]
}

fn resource_matches(
    data: &SimChunkData,
    kind: ResourceId,
    tx: i32,
    ty: i32,
    world_seed: u64,
) -> bool {
    let edge = CHUNK_EDGE as i32;
    let cell = if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        resource_at(data, tx, ty)
    } else {
        let gx = data.coord.cx * CHUNK_EDGE as i32 + tx;
        let gy = data.coord.cy * CHUNK_EDGE as i32 + ty;
        resource_at_global(gx, gy, data.layer, world_seed)
    };
    cell.kind == kind && cell.amount > 0
}

fn resource_hole_threshold(kind: ResourceId) -> f32 {
    match kind {
        RES_COAL => 0.18,
        RES_IRON => 0.15,
        RES_COPPER => 0.14,
        RES_STONE => 0.2,
        _ => 1.0,
    }
}

fn resource_overlay_weight(kind: ResourceId) -> f32 {
    match kind {
        RES_COAL => 0.68,
        RES_IRON => 0.64,
        RES_COPPER => 0.66,
        RES_STONE => 0.58,
        _ => 0.0,
    }
}

fn resource_highlight(kind: ResourceId) -> [u8; 4] {
    match kind {
        RES_COAL => [100, 106, 118, 255],
        RES_IRON => [221, 228, 232, 255],
        RES_COPPER => [238, 157, 86, 255],
        RES_STONE => [165, 165, 165, 255],
        _ => [0, 0, 0, 0],
    }
}

fn resource_noise(gx: i32, gy: i32, sub_x: usize, sub_y: usize, world_seed: u64, salt: u64) -> f32 {
    let mut z = world_seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    z ^= (gx as i64 as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z ^= (gy as i64 as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= (sub_x as u64).wrapping_mul(0x632b_e59b_d9b4_e019);
    z ^= (sub_y as u64).wrapping_mul(0x8cb9_2baa_72f3_d8dd);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    ((z >> 40) as u32 & 0xffff) as f32 / 65_535.0
}

fn strongest_edge_weight(
    predicate: impl Fn(i32, i32) -> bool,
    sub_x: usize,
    sub_y: usize,
    width: usize,
) -> f32 {
    let mut weight: f32 = 0.0;
    if predicate(-1, 0) && sub_x < width {
        weight = weight.max((width - sub_x) as f32 / width as f32);
    }
    if predicate(1, 0) && sub_x >= TERRAIN_TILE_PIXELS - width {
        weight = weight.max((sub_x + 1 - (TERRAIN_TILE_PIXELS - width)) as f32 / width as f32);
    }
    if predicate(0, 1) && sub_y < width {
        weight = weight.max((width - sub_y) as f32 / width as f32);
    }
    if predicate(0, -1) && sub_y >= TERRAIN_TILE_PIXELS - width {
        weight = weight.max((sub_y + 1 - (TERRAIN_TILE_PIXELS - width)) as f32 / width as f32);
    }
    weight.clamp(0.0, 1.0)
}

fn is_dirt_tile(tile: TileId) -> bool {
    matches!(tile, 4 | 5)
}

fn apply_tile_variant_tint(color: [u8; 4], tile: TileId) -> [u8; 4] {
    let amount = match tile {
        0 => -4,
        1 => 1,
        2 => 5,
        3 => 8,
        4 => -5,
        5 => 4,
        _ => 0,
    };
    adjust_color(color, amount)
}

fn detail_jitter(gx: i32, gy: i32, sub_x: usize, sub_y: usize, world_seed: u64) -> i16 {
    let mut z = world_seed;
    z ^= (gx as i64 as u64).wrapping_mul(0x9e3779b97f4a7c15);
    z ^= (gy as i64 as u64).wrapping_mul(0xc2b2ae3d27d4eb4f);
    z ^= (sub_x as u64).wrapping_mul(0x165667b19e3779f9);
    z ^= (sub_y as u64).wrapping_mul(0xd6e8feb86659fd93);
    let mixed = z ^ (z >> 30).wrapping_mul(0xbf58476d1ce4e5b9);
    ((mixed & 7) as i16) - 3
}

fn adjust_color(color: [u8; 4], amount: i16) -> [u8; 4] {
    let adjust = |value: u8| -> u8 { (value as i16 + amount).clamp(0, 255) as u8 };
    [
        adjust(color[0]),
        adjust(color[1]),
        adjust(color[2]),
        color[3],
    ]
}

pub(crate) fn tile_color(tile: TileId) -> [u8; 4] {
    match tile {
        0 => [58, 123, 70, 255],
        1 => [66, 132, 78, 255],
        2 => [72, 140, 82, 255],
        3 => [80, 148, 90, 255],
        4 => [132, 96, 60, 255],
        5 => [146, 106, 66, 255],
        6 => [46, 92, 166, 255],
        _ => [110, 110, 110, 255],
    }
}

pub(crate) fn shallow_water_color() -> [u8; 4] {
    [70, 120, 190, 255]
}

pub(crate) fn resource_color(kind: ResourceId) -> [u8; 4] {
    match kind {
        RES_IRON => [180, 180, 190, 255],
        RES_COPPER => [190, 120, 60, 255],
        RES_COAL => [30, 30, 30, 255],
        RES_STONE => [120, 120, 120, 255],
        _ => [0, 0, 0, 0],
    }
}

const FURNACE_WORLD_SPRITE: [&str; TERRAIN_TILE_PIXELS] = [
    "..kkkk..", ".kllllk.", "kmmmdmmk", "kmdrrdmk", "kmdyydmk", "kmmmmmmk", ".kddddk.", "..ssss..",
];

const CHEST_WORLD_SPRITE: [&str; TERRAIN_TILE_PIXELS] = [
    "..kkkk..", ".kllllk.", "kbbbbbbk", "kddmdddk", "kbbbbbbk", "kbdlldbk", ".kddddk.", "..ssss..",
];

const INSERTER_WORLD_SPRITE: [&str; TERRAIN_TILE_PIXELS] = [
    "....kk..", "...klyk.", "..klyyk.", ".klydkk.", "..kddk..", "..kyyk..", "..kkkk..", "...ss...",
];

const MINING_DRILL_WORLD_SPRITE: [&str; TERRAIN_TILE_PIXELS] = [
    "..kkkk..", ".kllmmk.", "kmmddmmk", "kmdccdmk", "kmmddmmk", ".kdkkdk.", ".kddddk.", "..ssss..",
];

pub(crate) fn apply_placed_sprite(
    base: [u8; 4],
    kind: PlacedId,
    sub_x: usize,
    sub_y: usize,
) -> [u8; 4] {
    let Some(rows) = placed_sprite_rows(kind) else {
        return blend_color(base, placed_color(kind), 0.88);
    };
    let Some(row) = rows.get(sub_y) else {
        return base;
    };
    let Some(key) = row.as_bytes().get(sub_x).copied() else {
        return base;
    };
    let overlay = placed_sprite_palette(kind, key);
    if overlay[3] == 0 {
        return base;
    }
    blend_color(
        base,
        [overlay[0], overlay[1], overlay[2], 255],
        overlay[3] as f32 / 255.0,
    )
}

fn placed_sprite_rows(kind: PlacedId) -> Option<&'static [&'static str; TERRAIN_TILE_PIXELS]> {
    match kind {
        PLACED_FURNACE => Some(&FURNACE_WORLD_SPRITE),
        PLACED_CHEST => Some(&CHEST_WORLD_SPRITE),
        PLACED_INSERTER => Some(&INSERTER_WORLD_SPRITE),
        PLACED_MINING_DRILL => Some(&MINING_DRILL_WORLD_SPRITE),
        _ => None,
    }
}

fn placed_sprite_palette(kind: PlacedId, key: u8) -> [u8; 4] {
    match kind {
        PLACED_FURNACE => match key {
            b'k' => [36, 33, 31, 255],
            b'd' => [78, 72, 67, 255],
            b'm' => [126, 119, 108, 255],
            b'l' => [176, 170, 156, 255],
            b'r' => [219, 77, 38, 255],
            b'y' => [255, 183, 52, 255],
            b's' => [22, 17, 13, 96],
            _ => [0, 0, 0, 0],
        },
        PLACED_CHEST => match key {
            b'k' => [49, 30, 17, 255],
            b'd' => [91, 54, 28, 255],
            b'b' => [139, 82, 39, 255],
            b'l' => [190, 119, 54, 255],
            b'm' => [218, 178, 83, 255],
            b's' => [22, 17, 13, 92],
            _ => [0, 0, 0, 0],
        },
        PLACED_INSERTER => match key {
            b'k' => [49, 45, 35, 255],
            b'd' => [121, 84, 29, 255],
            b'y' => [210, 149, 38, 255],
            b'l' => [246, 208, 88, 255],
            b's' => [22, 17, 13, 84],
            _ => [0, 0, 0, 0],
        },
        PLACED_MINING_DRILL => match key {
            b'k' => [35, 38, 39, 255],
            b'd' => [79, 84, 83, 255],
            b'm' => [116, 124, 124, 255],
            b'l' => [185, 193, 188, 255],
            b'c' => [224, 154, 54, 255],
            b's' => [22, 17, 13, 96],
            _ => [0, 0, 0, 0],
        },
        _ => [0, 0, 0, 0],
    }
}

pub(crate) fn placed_color(kind: PlacedId) -> [u8; 4] {
    match kind {
        PLACED_FURNACE => [90, 90, 100, 255],
        PLACED_CHEST => [150, 95, 55, 255],
        PLACED_INSERTER => [216, 168, 54, 255],
        PLACED_MINING_DRILL => [92, 118, 126, 255],
        _ => [0, 0, 0, 0],
    }
}

pub(crate) fn darken_color(color: [u8; 4]) -> [u8; 4] {
    let r = (color[0] as u16 * 4 / 5) as u8;
    let g = (color[1] as u16 * 4 / 5) as u8;
    let b = (color[2] as u16 * 4 / 5) as u8;
    [r, g, b, color[3]]
}

pub(crate) fn blend_color(base: [u8; 4], overlay: [u8; 4], overlay_weight: f32) -> [u8; 4] {
    let t = overlay_weight.clamp(0.0, 1.0);
    let blend = |b: u8, o: u8| -> u8 {
        let bf = b as f32;
        let of = o as f32;
        (bf * (1.0 - t) + of * t).round().clamp(0.0, 255.0) as u8
    };
    [
        blend(base[0], overlay[0]),
        blend(base[1], overlay[1]),
        blend(base[2], overlay[2]),
        base[3],
    ]
}

pub(crate) fn apply_jitter(color: [u8; 4], jitter: i8) -> [u8; 4] {
    let adjust = |value: u8| -> u8 {
        let v = value as i16 + jitter as i16;
        v.clamp(0, 255) as u8
    };
    [
        adjust(color[0]),
        adjust(color[1]),
        adjust(color[2]),
        color[3],
    ]
}

pub(crate) fn can_walk(world_pos: Vec2, config: &WorldRenderConfig, world_seed: u64) -> bool {
    let tx = (world_pos.x / config.tile_size).floor() as i32;
    let ty = (world_pos.y / config.tile_size).floor() as i32;
    let tile = terrain_tile_id(tx, ty, config.layer, world_seed);
    !is_water(tile)
}
