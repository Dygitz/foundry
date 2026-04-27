#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, gameplay::*, map::*, player::*, rendering::*, resources::*, storage::*,
    ui::*, world::*,
};

#[derive(Component)]
pub(crate) struct StorageStatusText;

#[derive(Component)]
pub(crate) struct ChunkRenderTag;

#[derive(Component)]
pub(crate) struct WorldStatsText;

#[derive(Component)]
pub(crate) struct CraftMenuText;

#[derive(Component)]
pub(crate) struct InventoryCellButton {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct InventoryItemCountText {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct RecipeButton {
    pub(crate) recipe_index: usize,
}

#[derive(Component)]
pub(crate) struct RecipeDetailText;

#[derive(Component)]
pub(crate) struct CraftingTabButton {
    pub(crate) tab: CraftingTab,
}

#[derive(Component)]
pub(crate) struct CraftingTabContent {
    pub(crate) tab: CraftingTab,
}

#[derive(Component)]
pub(crate) struct HotbarSlotButton {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct HotbarSlotIcon {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct HotbarSlotCountText {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct PickupFeedRoot;

#[derive(Component)]
pub(crate) struct PickupNoticeToast {
    pub(crate) item: ItemId,
    pub(crate) timer: Timer,
}

#[derive(Component)]
pub(crate) struct PickupNoticeText;

#[derive(Component)]
pub(crate) struct UiOverlay;

#[derive(Component)]
pub(crate) struct UiPanelRoot;

#[derive(Component)]
pub(crate) struct MinimapRoot;

#[derive(Component, Copy, Clone, PartialEq, Eq)]
pub(crate) enum MapSurfaceKind {
    Minimap,
    Full,
}

#[derive(Component, Copy, Clone)]
pub(crate) struct MapContent {
    pub(crate) kind: MapSurfaceKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct MapResourceCell {
    pub(crate) kind: ResourceId,
    pub(crate) amount: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ResourceNodeSummary {
    pub(crate) kind: ResourceId,
    pub(crate) total: u32,
}

#[derive(Component)]
pub(crate) struct CraftPanelText;

#[derive(Component)]
pub(crate) struct ChestPanel;

#[derive(Component)]
pub(crate) struct FurnacePanel;

#[derive(Component)]
pub(crate) struct ChestSlotButton {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct ChestSlotIcon {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct ChestSlotCountText {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct ChestDepositButton {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct ChestDepositIcon {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct ChestDepositCountText {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct FurnaceSlotButton {
    pub(crate) slot: FurnaceSlot,
}

#[derive(Component)]
pub(crate) struct FurnaceSlotIcon {
    pub(crate) slot: FurnaceSlot,
}

#[derive(Component)]
pub(crate) struct FurnaceSlotCountText {
    pub(crate) slot: FurnaceSlot,
}

#[derive(Component)]
pub(crate) struct FurnaceDepositButton {
    pub(crate) slot: FurnaceSlot,
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct FurnaceDepositIcon {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct FurnaceDepositCountText {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct FurnaceProgressBar;

#[derive(Component)]
pub(crate) struct DrillPanel;

#[derive(Component)]
pub(crate) struct DrillSlotButton {
    pub(crate) slot: DrillSlot,
}

#[derive(Component)]
pub(crate) struct DrillSlotIcon {
    pub(crate) slot: DrillSlot,
}

#[derive(Component)]
pub(crate) struct DrillSlotCountText {
    pub(crate) slot: DrillSlot,
}

#[derive(Component)]
pub(crate) struct DrillDepositButton {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct DrillDepositIcon {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct DrillDepositCountText {
    pub(crate) item: ItemId,
}

#[derive(Component)]
pub(crate) struct DrillProgressBar;

#[derive(Component)]
pub(crate) struct InserterPanel;

#[derive(Component)]
pub(crate) struct InserterDirectionText;

#[derive(Component)]
pub(crate) struct InserterSlotButton {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct InserterSlotIcon {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct InserterSlotCountText {
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct MiningFeedbackPanel;

#[derive(Component)]
pub(crate) struct MiningInstructionText;

#[derive(Component)]
pub(crate) struct MiningProgressFill;

#[derive(Component)]
pub(crate) struct MiningProgressText;

#[derive(Component)]
pub(crate) struct PickupFeedbackPanel;

#[derive(Component)]
pub(crate) struct PickupInstructionText;

#[derive(Component)]
pub(crate) struct PickupProgressFill;

#[derive(Component)]
pub(crate) struct PickupProgressText;

#[derive(Component)]
pub(crate) struct PlacementDirectionPanel;

#[derive(Component)]
pub(crate) struct PlacementDirectionText;

#[derive(Component)]
pub(crate) struct PlacementPreview;

#[derive(Component)]
pub(crate) struct Player;

#[derive(Component, Copy, Clone)]
pub(crate) struct Velocity(pub(crate) Vec2);

#[derive(Component, Copy, Clone)]
pub(crate) enum Facing {
    Up,
    Down,
    Left,
    Right,
}
