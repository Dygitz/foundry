#![allow(unused_imports)]
use bevy::app::PluginGroupBuilder;

use crate::imports::*;
use crate::{
    camera::FoundryCameraPlugin, gameplay::FoundryGameplayPlugin, map::FoundryMapPlugin,
    player::FoundryPlayerPlugin, resources::*, storage::FoundryStoragePlugin, ui::FoundryUiPlugin,
    world::FoundryWorldPlugin,
};

pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FoundryPlugins)
        .run();
}

pub(crate) struct FoundryPlugins;

impl PluginGroup for FoundryPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(FoundryCorePlugin)
            .add(FoundryStoragePlugin)
            .add(FoundryPlayerPlugin)
            .add(FoundryGameplayPlugin)
            .add(FoundryUiPlugin)
            .add(FoundryMapPlugin)
            .add(FoundryCameraPlugin)
            .add(FoundryWorldPlugin)
    }
}

struct FoundryCorePlugin;

impl Plugin for FoundryCorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.08, 0.75, 0.72)))
            .insert_resource(StorageConfig::default())
            .insert_resource(StorageStatus::default())
            .insert_resource(WorldSession::default())
            .insert_resource(WorldRenderConfig::default())
            .insert_resource(WorldRuntime::default())
            .insert_resource(ChunkCacheConfig::default())
            .insert_resource(PlayerConfig::default())
            .insert_resource(PlayerState::default())
            .insert_resource(PlayerStateSaveState::default())
            .insert_resource(PlayerStateLoadState::default())
            .insert_resource(PlacementState::default())
            .insert_resource(HotbarState::default())
            .insert_resource(UiState::default())
            .insert_resource(CraftingUiState::default())
            .insert_resource(ClickHighlight::default())
            .insert_resource(DebugConfig::default())
            .insert_resource(EvictionStats::default())
            .insert_resource(AutosaveState::default())
            .insert_resource(SaveQueue::default())
            .insert_resource(ChunkLoadState::default())
            .insert_resource(MapState::default())
            .insert_resource(MapLoadState::default())
            .insert_resource(MapSaveState::default())
            .insert_resource(RecoveryState::default())
            .add_event::<ChunkLoadRequest>()
            .add_event::<ChunkLoaded>()
            .add_event::<PickupNotice>()
            .configure_sets(
                Update,
                (UpdateSet::Input, UpdateSet::Ui, UpdateSet::World).chain(),
            );
    }
}
