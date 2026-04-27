#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, rendering::*, resources::*, storage::*,
    ui::*, world::*,
};

pub(crate) struct FoundryPlayerPlugin;

impl Plugin for FoundryPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                world_runtime_frame_counter_system,
                player_movement_system,
                player_visual_system,
                inventory_debug_input_system,
            )
                .in_set(UpdateSet::Input),
        );
    }
}

pub(crate) fn world_runtime_frame_counter_system(mut runtime: ResMut<WorldRuntime>) {
    runtime.advance_frame();
}

pub(crate) fn player_movement_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<PlayerConfig>,
    render_config: Res<WorldRenderConfig>,
    session: Res<WorldSession>,
    ui_state: Res<UiState>,
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Facing), With<Player>>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    let raw_direction = direction;
    let direction = direction.normalize_or_zero();
    let velocity = Velocity(direction * config.move_speed);
    let delta = velocity.0 * time.delta_secs();
    for (mut transform, mut current_velocity, mut facing) in &mut player_query {
        *current_velocity = velocity;
        let mut next = transform.translation;
        let next_x = next.x + delta.x;
        if can_walk(
            Vec2::new(next_x, next.y),
            &render_config,
            session.world_seed,
        ) {
            next.x = next_x;
        }
        let next_y = next.y + delta.y;
        if can_walk(
            Vec2::new(next.x, next_y),
            &render_config,
            session.world_seed,
        ) {
            next.y = next_y;
        }
        transform.translation = next;
        if raw_direction != Vec2::ZERO {
            if raw_direction.x.abs() >= raw_direction.y.abs() {
                *facing = if raw_direction.x > 0.0 {
                    Facing::Right
                } else {
                    Facing::Left
                };
            } else {
                *facing = if raw_direction.y > 0.0 {
                    Facing::Up
                } else {
                    Facing::Down
                };
            }
        }
    }
}

pub(crate) fn player_visual_system(
    sprites: Res<PlayerSpriteAssets>,
    mut player_query: Query<(&Facing, &mut Sprite), With<Player>>,
) {
    for (facing, mut sprite) in &mut player_query {
        match facing {
            Facing::Left => {
                sprite.image = sprites.side.clone();
                sprite.flip_x = true;
            }
            Facing::Right => {
                sprite.image = sprites.side.clone();
                sprite.flip_x = false;
            }
            Facing::Up => {
                sprite.image = sprites.up.clone();
                sprite.flip_x = false;
            }
            Facing::Down => {
                sprite.image = sprites.down.clone();
                sprite.flip_x = false;
            }
        }
    }
}

pub(crate) fn inventory_debug_input_system(
    _keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut player: ResMut<PlayerState>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    let _ = &mut player;
}
