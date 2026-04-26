#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    camera::*, components::*, gameplay::*, map::*, player::*, rendering::*, resources::*,
    storage::*, ui::*, world::*,
};

pub fn run() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.08, 0.75, 0.72)))
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
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .configure_sets(
            Update,
            (UpdateSet::Input, UpdateSet::Ui, UpdateSet::World).chain(),
        )
        .add_systems(Startup, (setup, init_storage))
        .add_systems(
            Update,
            (
                storage_init_pump_system,
                storage_recovery_pump_system,
                player_state_load_pump_system,
                autosave_flush_system,
                storage_status_text_system,
            ),
        )
        .add_systems(
            Update,
            (
                world_runtime_frame_counter_system,
                player_movement_system,
                player_visual_system,
                craft_menu_toggle_system,
                map_toggle_system,
                full_map_input_system,
                placement_select_system,
                hotbar_input_system,
                ui_close_system,
                inventory_debug_input_system,
            )
                .in_set(UpdateSet::Input),
        )
        .add_systems(Update, (mining_input_system,).in_set(UpdateSet::Input))
        .add_systems(
            Update,
            (
                ui_visibility_system,
                hotbar_auto_assign_system,
                hotbar_button_system,
                hotbar_ui_system,
                craft_menu_text_system,
                inventory_cell_hover_system,
                crafting_recipe_button_system,
                crafting_panel_text_system,
                crafting_recipe_visual_system,
                pickup_notice_spawn_system,
                pickup_notice_lifetime_system,
                chest_ui_system,
                furnace_ui_system,
                minimap_visibility_system,
                map_load_pump_system,
                map_save_system,
                map_ui_render_system,
                chest_button_system,
                furnace_button_system,
            )
                .in_set(UpdateSet::Ui),
        )
        .add_systems(Update, (player_state_save_system,).in_set(UpdateSet::Ui))
        .add_systems(
            Update,
            (
                camera_follow_system,
                camera_zoom_system,
                active_area_chunk_request_system,
                chunk_load_pump_system,
                chunk_loaded_system,
                furnace_smelting_system,
                chunk_eviction_system,
                world_stats_text_system,
            )
                .in_set(UpdateSet::World),
        )
        .run();
}
