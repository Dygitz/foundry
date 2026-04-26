#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, components::*, gameplay::*, map::*, player::*, rendering::*, resources::*, storage::*,
    ui::*, world::*,
};

pub(crate) struct FoundryCameraPlugin;

impl Plugin for FoundryCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (camera_follow_system, camera_zoom_system).in_set(UpdateSet::World),
        );
    }
}

pub(crate) fn camera_follow_system(
    time: Res<Time>,
    config: Res<PlayerConfig>,
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let mut target = player_transform.translation;
    target.z = camera_transform.translation.z;
    let t = 1.0 - (-config.camera_follow_lerp * time.delta_secs()).exp();
    camera_transform.translation = camera_transform.translation.lerp(target, t);
}

pub(crate) fn camera_zoom_system(
    mut scroll: EventReader<MouseWheel>,
    ui_state: Res<UiState>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    let Ok(mut projection) = camera_query.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = &mut *projection else {
        return;
    };

    for ev in scroll.read() {
        let factor = (1.0 - ev.y * 0.1).clamp(0.7, 1.3);
        ortho.scale = (ortho.scale * factor).clamp(0.25, 0.9);
    }
}
