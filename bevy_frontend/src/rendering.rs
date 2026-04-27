#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, player::*, resources::*, storage::*,
    ui::*, world::*,
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

pub(crate) fn build_chunk_image(
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) -> Image {
    let pixels = chunk_pixels(data, config, world_seed, highlight);
    let padded_edge = CHUNK_EDGE as u32 + 2;
    let mut image = Image::new_fill(
        Extent3d {
            width: padded_edge,
            height: padded_edge,
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
    let edge = CHUNK_EDGE as f32;
    Rect::from_corners(Vec2::new(1.0, 1.0), Vec2::new(edge + 1.0, edge + 1.0))
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
    let padded_edge = edge + 2;
    let mut pixels = Vec::with_capacity(padded_edge * padded_edge * 4);

    for oy in 0..padded_edge {
        let base_ty = if oy == 0 {
            0
        } else if oy > edge {
            edge - 1
        } else {
            oy - 1
        };
        let ty = edge - 1 - base_ty;
        let interior_y = oy as i32 - 1;
        for ox in 0..padded_edge {
            let tx = if ox == 0 {
                0
            } else if ox > edge {
                edge - 1
            } else {
                ox - 1
            };
            let interior_x = ox as i32 - 1;
            let tile = tile_at(data, tx as i32, ty as i32, world_seed);
            let mut color = if tile == WATER_TILE {
                let neighbor_is_land =
                    !is_water(tile_at(data, tx as i32 - 1, ty as i32, world_seed))
                        || !is_water(tile_at(data, tx as i32 + 1, ty as i32, world_seed))
                        || !is_water(tile_at(data, tx as i32, ty as i32 - 1, world_seed))
                        || !is_water(tile_at(data, tx as i32, ty as i32 + 1, world_seed));
                if neighbor_is_land {
                    shallow_water_color()
                } else {
                    tile_color(tile)
                }
            } else {
                tile_color(tile)
            };
            let gx = data.coord.cx * CHUNK_EDGE as i32 + tx as i32;
            let gy = data.coord.cy * CHUNK_EDGE as i32 + ty as i32;
            let jitter = tile_jitter(gx, gy, world_seed, tile);
            color = apply_jitter(color, jitter);
            let resource = resource_at(data, tx as i32, ty as i32);
            if resource.kind != RES_NONE && resource.amount > 0 {
                let overlay = resource_color(resource.kind);
                color = blend_color(color, overlay, 0.85);
            }
            let placed = placed_at(data, tx as i32, ty as i32);
            if placed.kind != PLACED_NONE {
                let overlay = placed_color(placed.kind);
                color = blend_color(color, overlay, 0.9);
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
                    color = [220, 40, 40, 255];
                }
            }
            pixels.extend_from_slice(&color);
        }
    }
    pixels
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
