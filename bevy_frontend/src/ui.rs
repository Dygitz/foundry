#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, player::*, rendering::*, resources::*,
    storage::*, world::*,
};

pub(crate) struct FoundryUiPlugin;

impl Plugin for FoundryUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (craft_menu_toggle_system, ui_close_system).in_set(UpdateSet::Input),
            )
            .add_systems(Update, (storage_status_text_system,))
            .add_systems(
                Update,
                (
                    ui_visibility_system,
                    hotbar_auto_assign_system,
                    hotbar_button_system,
                    hotbar_ui_system,
                    craft_menu_text_system,
                    mining_feedback_ui_system,
                    structure_pickup_feedback_ui_system,
                    placement_direction_ui_system,
                    inventory_cell_hover_system,
                    crafting_recipe_button_system,
                    crafting_panel_text_system,
                    crafting_recipe_visual_system,
                    pickup_notice_spawn_system,
                    pickup_notice_lifetime_system,
                    chest_ui_system,
                    furnace_ui_system,
                    inserter_ui_system,
                )
                    .in_set(UpdateSet::Ui),
            )
            .add_systems(Update, (world_stats_text_system,).in_set(UpdateSet::World));
    }
}

pub(crate) fn setup(
    mut commands: Commands,
    config: Res<WorldRenderConfig>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut camera = commands.spawn(Camera2d);
    camera.insert(Projection::Orthographic(OrthographicProjection {
        scale: 0.35,
        ..OrthographicProjection::default_2d()
    }));
    let player_texture = images.add(build_player_image());
    commands.spawn((
        Sprite {
            image: player_texture,
            custom_size: Some(Vec2::splat(config.tile_size * 0.95)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        Velocity(Vec2::ZERO),
        Facing::Down,
        Player,
    ));
    let icons = build_ui_icon_assets(&mut images);
    commands.insert_resource(icons.clone());
    spawn_game_hud(&mut commands, &icons);
    spawn_pickup_feed(&mut commands);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Visibility::Hidden,
            UiOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                UiPanelRoot,
            ));
        });
    commands
        .spawn((
            Node {
                width: Val::Px(MINIMAP_OUTER_SIZE),
                height: Val::Px(MINIMAP_OUTER_SIZE),
                position_type: PositionType::Absolute,
                right: Val::Px(MINIMAP_MARGIN),
                bottom: Val::Px(MINIMAP_MARGIN),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.44, 0.48, 0.42, 0.95)),
            BorderRadius::all(Val::Px(4.0)),
            hud_shadow(),
            MinimapRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(MINIMAP_SIZE),
                    height: Val::Px(MINIMAP_SIZE),
                    position_type: PositionType::Relative,
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(map_unknown_color()),
                MapContent {
                    kind: MapSurfaceKind::Minimap,
                },
            ));
        });
}

pub(crate) fn spawn_pickup_feed(commands: &mut Commands) {
    commands.spawn((
        Node {
            width: Val::Px(220.0),
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(76.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        PickupFeedRoot,
    ));
}

pub(crate) fn build_ui_icon_assets(images: &mut Assets<Image>) -> UiIconAssets {
    UiIconAssets {
        empty: images.add(build_hud_icon_image(HudIconKind::Empty)),
        heart: images.add(build_hud_icon_image(HudIconKind::Heart)),
        stone: images.add(build_hud_icon_image(HudIconKind::Stone)),
        copper_ore: images.add(build_hud_icon_image(HudIconKind::CopperOre)),
        coal: images.add(build_hud_icon_image(HudIconKind::Coal)),
        iron_ore: images.add(build_hud_icon_image(HudIconKind::IronOre)),
        iron_plate: images.add(build_hud_icon_image(HudIconKind::IronPlate)),
        copper_plate: images.add(build_hud_icon_image(HudIconKind::CopperPlate)),
        furnace: images.add(build_hud_icon_image(HudIconKind::Furnace)),
        chest: images.add(build_hud_icon_image(HudIconKind::Chest)),
        inserter: images.add(build_hud_icon_image(HudIconKind::Inserter)),
        crafting: images.add(build_hud_icon_image(HudIconKind::Anvil)),
    }
}

pub(crate) fn spawn_game_hud(commands: &mut Commands, icons: &UiIconAssets) {
    commands
        .spawn(Node {
            width: Val::Px(422.0),
            max_width: Val::Percent(92.0),
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(14.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(hud_panel_color()),
                BorderColor(hud_border_color()),
                BorderRadius::all(Val::Px(8.0)),
                hud_shadow(),
            ))
            .with_children(|panel| {
                spawn_health_bar(panel, icons.heart.clone());
            });
        });

    commands
        .spawn(Node {
            width: Val::Px(272.0),
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|root| {
            spawn_action_prompt(root, icons.crafting.clone(), "Crafting", "[E]");
            spawn_placement_direction_panel(root);
            spawn_pickup_feedback_panel(root);
            spawn_mining_feedback_panel(root);
        });

    spawn_hotbar(commands, icons);
}

pub(crate) fn spawn_health_bar(parent: &mut ChildSpawnerCommands, icon: Handle<Image>) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(28.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn((
                Node {
                    height: Val::Px(26.0),
                    flex_grow: 1.0,
                    position_type: PositionType::Relative,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.03, 0.02, 0.72)),
                BorderRadius::all(Val::Px(9.0)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.78, 0.28)),
                    BorderRadius::all(Val::Px(9.0)),
                ));
                bar.spawn((
                    Text::new("100/100"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.98, 0.98, 0.95)),
                    TextLayout::new_with_justify(JustifyText::Center),
                    TextShadow {
                        offset: Vec2::new(1.0, 1.0),
                        color: Color::srgba(0.0, 0.0, 0.0, 0.75),
                    },
                    Node {
                        width: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(3.0),
                        ..default()
                    },
                ));
            });
        });
}

pub(crate) fn spawn_action_prompt(
    parent: &mut ChildSpawnerCommands,
    icon: Handle<Image>,
    label: &'static str,
    key: &'static str,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(54.0),
                align_items: AlignItems::Center,
                padding: UiRect::new(Val::Px(14.0), Val::Px(12.0), Val::Px(0.0), Val::Px(0.0)),
                column_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
        ))
        .with_children(|row| {
            row.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            let mut label_entity = row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.96, 0.94)),
                hud_text_shadow(),
            ));
            label_entity.insert(CraftMenuText);
            row.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            row.spawn((
                Text::new(key),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.93, 0.9)),
                hud_text_shadow(),
            ));
        });
}

pub(crate) fn spawn_hotbar(commands: &mut Commands, icons: &UiIconAssets) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(22.0),
            bottom: Val::Px(20.0),
            height: Val::Px(58.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|bar| {
            for (index, label) in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]
                .into_iter()
                .enumerate()
            {
                bar.spawn((
                    Button,
                    Node {
                        width: Val::Px(56.0),
                        height: Val::Px(56.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        position_type: PositionType::Relative,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.11, 0.09, 0.72)),
                    BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.16)),
                    BorderRadius::all(Val::Px(7.0)),
                    hud_shadow(),
                    HotbarSlotButton { index },
                ))
                .with_children(|slot| {
                    slot.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.96, 0.96, 0.94, 0.86)),
                        hud_text_shadow(),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(6.0),
                            top: Val::Px(4.0),
                            ..default()
                        },
                    ));
                    slot.spawn((
                        ImageNode::new(icons.empty.clone()),
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            ..default()
                        },
                        HotbarSlotIcon { index },
                    ));
                    slot.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.96, 0.94)),
                        hud_text_shadow(),
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(5.0),
                            bottom: Val::Px(3.0),
                            ..default()
                        },
                        HotbarSlotCountText { index },
                    ));
                });
            }
        });
}

pub(crate) fn spawn_mining_feedback_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
            Visibility::Hidden,
            MiningFeedbackPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.94, 0.9)),
                hud_text_shadow(),
                MiningInstructionText,
            ));
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(18.0),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.03, 0.02, 0.72)),
                    BorderRadius::all(Val::Px(9.0)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.78, 0.3)),
                        BorderRadius::all(Val::Px(9.0)),
                        MiningProgressFill,
                    ));
                });
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.94, 0.94, 0.9, 0.82)),
                hud_text_shadow(),
                MiningProgressText,
            ));
        });
}

pub(crate) fn spawn_pickup_feedback_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
            Visibility::Hidden,
            PickupFeedbackPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.91, 0.78)),
                hud_text_shadow(),
                PickupInstructionText,
            ));
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(18.0),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.03, 0.02, 0.72)),
                    BorderRadius::all(Val::Px(9.0)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.86, 0.58, 0.18)),
                        BorderRadius::all(Val::Px(9.0)),
                        PickupProgressFill,
                    ));
                });
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.94, 0.94, 0.9, 0.82)),
                hud_text_shadow(),
                PickupProgressText,
            ));
        });
}

pub(crate) fn spawn_placement_direction_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
            Visibility::Hidden,
            PlacementDirectionPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.85, 0.56)),
                hud_text_shadow(),
                PlacementDirectionText,
            ));
        });
}

pub(crate) fn hud_panel_color() -> Color {
    Color::srgba(0.06, 0.055, 0.045, 0.78)
}

pub(crate) fn hud_border_color() -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.08)
}

pub(crate) fn hud_shadow() -> BoxShadow {
    BoxShadow::new(
        Color::srgba(0.0, 0.0, 0.0, 0.35),
        Val::Px(0.0),
        Val::Px(5.0),
        Val::Px(0.0),
        Val::Px(10.0),
    )
}

pub(crate) fn hud_text_shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::new(1.0, 1.0),
        color: Color::srgba(0.0, 0.0, 0.0, 0.7),
    }
}

pub(crate) fn spawn_map_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(map_unknown_color()),
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                MapContent {
                    kind: MapSurfaceKind::Full,
                },
            ));
        });
}

pub(crate) fn spawn_crafting_panel(parent: &mut ChildSpawnerCommands, icons: &UiIconAssets) {
    parent
        .spawn((
            Node {
                width: Val::Px(760.0),
                max_width: Val::Percent(92.0),
                min_height: Val::Px(420.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(Val::Px(16.0)),
                column_gap: Val::Px(16.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    width: Val::Px(280.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|inventory| {
                    inventory.spawn((
                        Text::new("Inventory"),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.95, 0.93)),
                        hud_text_shadow(),
                    ));
                    inventory
                        .spawn(Node {
                            display: Display::Flex,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|grid| {
                            for item in INVENTORY_ITEMS {
                                spawn_inventory_cell(grid, icons, item);
                            }
                        });
                });

            panel
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|crafting| {
                    crafting.spawn((
                        Text::new("Crafting"),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.95, 0.93)),
                        hud_text_shadow(),
                    ));
                    crafting
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(96.0),
                            column_gap: Val::Px(10.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|recipes| {
                            for recipe_index in 0..RECIPES.len() {
                                spawn_recipe_button(recipes, icons, recipe_index);
                            }
                        });
                    crafting.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.87)),
                        hud_text_shadow(),
                        CraftPanelText,
                        RecipeDetailText,
                    ));
                });
        });
}

pub(crate) fn spawn_inventory_cell(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    item: ItemId,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.11, 0.09, 0.72)),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.1)),
            BorderRadius::all(Val::Px(5.0)),
            InventoryCellButton { item },
        ))
        .with_children(|cell| {
            cell.spawn((
                ImageNode::new(icons.for_item(item)),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            cell.spawn((
                Text::new("0"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.94, 0.91)),
                hud_text_shadow(),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(5.0),
                    bottom: Val::Px(3.0),
                    ..default()
                },
                InventoryItemCountText { item },
            ));
        });
}

pub(crate) fn spawn_recipe_button(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    recipe_index: usize,
) {
    let Some(recipe) = recipe_for_index(recipe_index) else {
        return;
    };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(72.0),
                height: Val::Px(72.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.11, 0.09, 0.72)),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
            BorderRadius::all(Val::Px(6.0)),
            RecipeButton { recipe_index },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icons.for_item(recipe.output)),
                Node {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    ..default()
                },
            ));
        });
}

pub(crate) fn spawn_chest_panel(parent: &mut ChildSpawnerCommands, icons: &UiIconAssets) {
    parent
        .spawn((
            Node {
                width: Val::Px(584.0),
                max_width: Val::Percent(92.0),
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(14.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
            ChestPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Chest"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.93)),
                hud_text_shadow(),
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    column_gap: Val::Px(24.0),
                    ..default()
                })
                .with_children(|body| {
                    body.spawn(Node {
                        width: Val::Px(256.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|section| {
                        spawn_panel_section_label(section, "Stored");
                        section
                            .spawn(Node {
                                display: Display::Flex,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(8.0),
                                row_gap: Val::Px(8.0),
                                ..default()
                            })
                            .with_children(|grid| {
                                for index in 0..CHEST_SLOT_COUNT {
                                    spawn_chest_slot_cell(grid, icons, index);
                                }
                            });
                    });

                    body.spawn(Node {
                        width: Val::Px(256.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|section| {
                        spawn_panel_section_label(section, "Inventory");
                        section
                            .spawn(Node {
                                display: Display::Flex,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(8.0),
                                row_gap: Val::Px(8.0),
                                ..default()
                            })
                            .with_children(|grid| {
                                for item in INVENTORY_ITEMS {
                                    spawn_chest_deposit_cell(grid, icons, item);
                                }
                            });
                    });
                });
        });
}

pub(crate) fn spawn_chest_slot_cell(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    index: usize,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(58.0),
                height: Val::Px(58.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            storage_cell_background(false, false),
            storage_cell_border(false),
            BorderRadius::all(Val::Px(5.0)),
            ChestSlotButton { index },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icons.empty.clone()),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                ChestSlotIcon { index },
            ));
            button.spawn((
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.94, 0.91)),
                hud_text_shadow(),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(5.0),
                    bottom: Val::Px(3.0),
                    ..default()
                },
                ChestSlotCountText { index },
            ));
        });
}

pub(crate) fn spawn_chest_deposit_cell(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    item: ItemId,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(58.0),
                height: Val::Px(58.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            storage_cell_background(false, false),
            storage_cell_border(false),
            BorderRadius::all(Val::Px(5.0)),
            ChestDepositButton { item },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icons.for_item(item)),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                ChestDepositIcon { item },
            ));
            button.spawn((
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.94, 0.91)),
                hud_text_shadow(),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(5.0),
                    bottom: Val::Px(3.0),
                    ..default()
                },
                ChestDepositCountText { item },
            ));
        });
}

pub(crate) fn spawn_furnace_panel(parent: &mut ChildSpawnerCommands, icons: &UiIconAssets) {
    parent
        .spawn((
            Node {
                width: Val::Px(430.0),
                max_width: Val::Percent(92.0),
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(14.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
            FurnacePanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Furnace"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.93)),
                hud_text_shadow(),
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(18.0),
                    ..default()
                })
                .with_children(|slots| {
                    spawn_furnace_slot_cell(slots, icons, FurnaceSlot::Input, "Input");
                    spawn_furnace_slot_cell(slots, icons, FurnaceSlot::Fuel, "Fuel");
                    spawn_furnace_slot_cell(slots, icons, FurnaceSlot::Output, "Output");
                });
            panel
                .spawn((
                    Node {
                        width: Val::Px(FURNACE_PROGRESS_BAR_WIDTH),
                        height: Val::Px(10.0),
                        align_self: AlignSelf::Center,
                        overflow: Overflow::clip(),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.03, 0.04, 0.035, 0.78)),
                    BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.32, 0.78, 0.28)),
                        FurnaceProgressBar,
                    ));
                });
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|section| {
                    spawn_panel_section_label(section, "Inventory");
                    section
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_furnace_deposit_cell(
                                row,
                                icons,
                                FurnaceSlot::Input,
                                ITEM_IRON_ORE,
                            );
                            spawn_furnace_deposit_cell(
                                row,
                                icons,
                                FurnaceSlot::Input,
                                ITEM_COPPER_ORE,
                            );
                            spawn_furnace_deposit_cell(row, icons, FurnaceSlot::Fuel, ITEM_COAL);
                        });
                });
        });
}

pub(crate) fn spawn_furnace_slot_cell(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    slot: FurnaceSlot,
    label: &'static str,
) {
    parent
        .spawn(Node {
            width: Val::Px(68.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|stack| {
            stack.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.84, 0.8)),
                hud_text_shadow(),
            ));
            stack
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(58.0),
                        height: Val::Px(58.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        position_type: PositionType::Relative,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    storage_cell_background(false, false),
                    storage_cell_border(false),
                    BorderRadius::all(Val::Px(5.0)),
                    FurnaceSlotButton { slot },
                ))
                .with_children(|button| {
                    button.spawn((
                        ImageNode::new(icons.empty.clone()),
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        FurnaceSlotIcon { slot },
                    ));
                    button.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.94, 0.94, 0.91)),
                        hud_text_shadow(),
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(5.0),
                            bottom: Val::Px(3.0),
                            ..default()
                        },
                        FurnaceSlotCountText { slot },
                    ));
                });
        });
}

pub(crate) fn spawn_furnace_deposit_cell(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    slot: FurnaceSlot,
    item: ItemId,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(58.0),
                height: Val::Px(58.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            storage_cell_background(false, false),
            storage_cell_border(false),
            BorderRadius::all(Val::Px(5.0)),
            FurnaceDepositButton { slot, item },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icons.for_item(item)),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                FurnaceDepositIcon { item },
            ));
            button.spawn((
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.94, 0.91)),
                hud_text_shadow(),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(5.0),
                    bottom: Val::Px(3.0),
                    ..default()
                },
                FurnaceDepositCountText { item },
            ));
        });
}

pub(crate) fn spawn_inserter_panel(parent: &mut ChildSpawnerCommands, icons: &UiIconAssets) {
    parent
        .spawn((
            Node {
                width: Val::Px(342.0),
                max_width: Val::Percent(92.0),
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(14.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(hud_panel_color()),
            BorderColor(hud_border_color()),
            BorderRadius::all(Val::Px(8.0)),
            hud_shadow(),
            InserterPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Inserter"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.93)),
                hud_text_shadow(),
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.94, 0.9, 0.74, 0.92)),
                hud_text_shadow(),
                InserterDirectionText,
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|section| {
                    spawn_panel_section_label(section, "Buffer");
                    section
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for index in 0..INSERTER_SLOT_COUNT {
                                spawn_inserter_slot_cell(row, icons, index);
                            }
                        });
                });
        });
}

pub(crate) fn spawn_inserter_slot_cell(
    parent: &mut ChildSpawnerCommands,
    icons: &UiIconAssets,
    index: usize,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(58.0),
                height: Val::Px(58.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            storage_cell_background(false, false),
            storage_cell_border(false),
            BorderRadius::all(Val::Px(5.0)),
            InserterSlotButton { index },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icons.empty.clone()),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                InserterSlotIcon { index },
            ));
            button.spawn((
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.94, 0.91)),
                hud_text_shadow(),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(5.0),
                    bottom: Val::Px(3.0),
                    ..default()
                },
                InserterSlotCountText { index },
            ));
        });
}

pub(crate) fn spawn_panel_section_label(parent: &mut ChildSpawnerCommands, label: &'static str) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.84, 0.8)),
        hud_text_shadow(),
    ));
}

pub(crate) fn storage_cell_background(occupied: bool, hovered: bool) -> BackgroundColor {
    BackgroundColor(match (occupied, hovered) {
        (true, true) => Color::srgba(0.16, 0.21, 0.15, 0.86),
        (true, false) => Color::srgba(0.11, 0.15, 0.11, 0.78),
        (false, true) => Color::srgba(0.11, 0.13, 0.11, 0.72),
        (false, false) => Color::srgba(0.06, 0.08, 0.07, 0.62),
    })
}

pub(crate) fn storage_cell_border(occupied: bool) -> BorderColor {
    BorderColor(if occupied {
        Color::srgba(0.82, 0.92, 0.76, 0.38)
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.12)
    })
}

pub(crate) fn item_count_label(count: u32) -> String {
    if count == 0 {
        String::new()
    } else {
        count.to_string()
    }
}

pub(crate) fn inserter_direction_label(direction: InserterDirection) -> String {
    format!("{} {}", direction.label(), direction.arrow())
}

pub(crate) fn furnace_slot(furnace: Option<&FurnaceRecord>, slot: FurnaceSlot) -> Option<Slot> {
    furnace.map(|furnace| match slot {
        FurnaceSlot::Input => furnace.state.input,
        FurnaceSlot::Fuel => furnace.state.fuel,
        FurnaceSlot::Output => furnace.state.output,
    })
}

pub(crate) fn craft_menu_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    match ui_state.mode {
        UiMode::None => ui_state.mode = UiMode::Crafting,
        UiMode::Crafting => ui_state.mode = UiMode::None,
        _ => {}
    }
}

pub(crate) fn ui_close_system(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut ui_state: ResMut<UiState>,
    mut placement: ResMut<PlacementState>,
    mut hotbar: ResMut<HotbarState>,
) {
    if keys.just_pressed(KeyCode::Escape) || buttons.just_pressed(MouseButton::Right) {
        if ui_state.mode == UiMode::None {
            placement.selected = None;
            hotbar.selected_slot = None;
        } else {
            ui_state.mode = UiMode::None;
        }
    }
}

pub(crate) fn ui_visibility_system(
    mut commands: Commands,
    ui_state: Res<UiState>,
    icons: Res<UiIconAssets>,
    mut overlay_query: Query<&mut Visibility, With<UiOverlay>>,
    panel_query: Query<Entity, With<UiPanelRoot>>,
    children_query: Query<&Children, With<UiPanelRoot>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    let Ok(mut overlay_visibility) = overlay_query.single_mut() else {
        return;
    };
    let Ok(panel_entity) = panel_query.single() else {
        return;
    };

    if let Ok(children) = children_query.get(panel_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    match ui_state.mode {
        UiMode::None => {
            *overlay_visibility = Visibility::Hidden;
        }
        UiMode::Map => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_map_panel(parent));
        }
        UiMode::Crafting => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_crafting_panel(parent, &icons));
        }
        UiMode::Chest { .. } => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_chest_panel(parent, &icons));
        }
        UiMode::Furnace { .. } => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_furnace_panel(parent, &icons));
        }
        UiMode::Inserter { .. } => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_inserter_panel(parent, &icons));
        }
    }
}

pub(crate) fn hotbar_auto_assign_system(
    player: Res<PlayerState>,
    mut hotbar: ResMut<HotbarState>,
    mut placement: ResMut<PlacementState>,
) {
    for item in PLACEABLE_ITEMS {
        if player.inventory.count(item) > 0 {
            hotbar.assign_item(item);
        }
    }

    if let Some(slot) = hotbar.selected_slot {
        let selected = hotbar.slots.get(slot).and_then(|slot| *slot);
        match selected {
            Some(item) if player.inventory.count(item) > 0 && is_placeable_item(item) => {
                placement.selected = Some(item);
            }
            _ => {
                hotbar.selected_slot = None;
                placement.selected = None;
            }
        }
    }
}

pub(crate) fn hotbar_button_system(
    player: Res<PlayerState>,
    mut hotbar: ResMut<HotbarState>,
    mut placement: ResMut<PlacementState>,
    mut query: Query<(&Interaction, &HotbarSlotButton), Changed<Interaction>>,
) {
    for (interaction, button) in &mut query {
        if *interaction == Interaction::Pressed {
            select_hotbar_slot(button.index, &player.inventory, &mut hotbar, &mut placement);
        }
    }
}

pub(crate) fn hotbar_ui_system(
    player: Res<PlayerState>,
    hotbar: Res<HotbarState>,
    icons: Res<UiIconAssets>,
    mut button_query: Query<(
        &HotbarSlotButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut icon_query: Query<(&HotbarSlotIcon, &mut ImageNode)>,
    mut count_query: Query<(&HotbarSlotCountText, &mut Text)>,
) {
    for (button, interaction, mut background, mut border) in &mut button_query {
        let selected = hotbar.selected_slot == Some(button.index);
        let occupied = hotbar.slots.get(button.index).and_then(|slot| *slot);
        let count = occupied
            .map(|item| player.inventory.count(item))
            .unwrap_or(0);
        *background = BackgroundColor(match (selected, *interaction) {
            (true, _) => Color::srgba(0.15, 0.22, 0.14, 0.84),
            (false, Interaction::Hovered) => Color::srgba(0.12, 0.15, 0.12, 0.82),
            _ => Color::srgba(0.08, 0.11, 0.09, 0.72),
        });
        *border = BorderColor(if selected {
            Color::srgb(0.98, 0.98, 0.95)
        } else if occupied.is_some() && count > 0 {
            Color::srgba(0.8, 0.92, 0.72, 0.36)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.16)
        });
    }

    for (slot_icon, mut image) in &mut icon_query {
        if let Some(item) = hotbar.slots.get(slot_icon.index).and_then(|slot| *slot) {
            image.image = icons.for_item(item);
            image.color = if player.inventory.count(item) > 0 {
                Color::WHITE
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.35)
            };
        } else {
            image.image = icons.empty.clone();
            image.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        }
    }

    for (slot_text, mut text) in &mut count_query {
        let label = hotbar
            .slots
            .get(slot_text.index)
            .and_then(|slot| *slot)
            .map(|item| player.inventory.count(item).to_string())
            .unwrap_or_default();
        *text = Text::new(label);
    }
}

pub(crate) fn craft_menu_text_system(
    ui_state: Res<UiState>,
    mut query: Query<&mut Text, With<CraftMenuText>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    let label = if matches!(ui_state.mode, UiMode::Crafting) {
        "Close Crafting"
    } else {
        "Crafting"
    };
    for mut text in &mut query {
        *text = Text::new(label.to_string());
    }
}

pub(crate) fn mining_feedback_ui_system(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    config: Res<WorldRenderConfig>,
    session: Res<WorldSession>,
    map: Res<MapState>,
    ui_state: Res<UiState>,
    pickup_state: Res<StructurePickupState>,
    mut mining_feedback: ResMut<MiningFeedbackState>,
    mut panel_query: Query<&mut Visibility, With<MiningFeedbackPanel>>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<MiningInstructionText>>,
        Query<&mut Text, With<MiningProgressText>>,
    )>,
    mut bar_query: Query<&mut Node, With<MiningProgressFill>>,
) {
    let mut target = None;
    if ui_state.mode == UiMode::None && pickup_state.target.is_none() {
        if let (Ok(window), Ok((camera, camera_transform))) =
            (windows.single(), camera_query.single())
        {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    let tile_x = (world_pos.x / config.tile_size).floor() as i32;
                    let tile_y = (world_pos.y / config.tile_size).floor() as i32;
                    if let Some(resource) =
                        map_resource_at(&map, &session, config.layer, tile_x, tile_y)
                    {
                        target = Some((tile_x, tile_y, resource));
                    }
                }
            }
        }
    }

    let mut instruction = String::new();
    let mut details = String::new();
    let mut ratio = 0.0f32;

    if let Some((tile_x, tile_y, resource)) = target {
        for mut visibility in &mut panel_query {
            *visibility = Visibility::Visible;
        }
        if mining_feedback.tracked_tile != Some((tile_x, tile_y))
            || mining_feedback.tracked_resource_kind != resource.kind
        {
            mining_feedback.tracked_tile = Some((tile_x, tile_y));
            mining_feedback.tracked_resource_kind = resource.kind;
            mining_feedback.tracked_max_amount = resource.amount;
        } else {
            mining_feedback.tracked_max_amount =
                mining_feedback.tracked_max_amount.max(resource.amount);
        }
        let max_amount = mining_feedback.tracked_max_amount.max(1);
        ratio = (resource.amount as f32 / max_amount as f32).clamp(0.0, 1.0);
        let resource_name = resource_display_name(resource.kind);
        instruction = format!("Mining {resource_name}");
        details = format!("{resource_name} left: {}/{}", resource.amount, max_amount);
    } else {
        for mut visibility in &mut panel_query {
            *visibility = Visibility::Hidden;
        }
        mining_feedback.tracked_tile = None;
        mining_feedback.tracked_resource_kind = RES_NONE;
        mining_feedback.tracked_max_amount = 0;
    }

    for mut text in &mut text_queries.p0() {
        *text = Text::new(instruction.clone());
    }
    for mut node in &mut bar_query {
        node.width = Val::Percent(ratio * 100.0);
    }
    for mut text in &mut text_queries.p1() {
        *text = Text::new(details.clone());
    }
}

pub(crate) fn structure_pickup_feedback_ui_system(
    pickup_state: Res<StructurePickupState>,
    mut panel_query: Query<&mut Visibility, With<PickupFeedbackPanel>>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<PickupInstructionText>>,
        Query<&mut Text, With<PickupProgressText>>,
    )>,
    mut bar_query: Query<&mut Node, With<PickupProgressFill>>,
) {
    let Some(target) = pickup_state.target.as_ref() else {
        for mut visibility in &mut panel_query {
            *visibility = Visibility::Hidden;
        }
        for mut node in &mut bar_query {
            node.width = Val::Percent(0.0);
        }
        return;
    };

    for mut visibility in &mut panel_query {
        *visibility = Visibility::Visible;
    }

    let item = placed_kind_to_item(target.kind).unwrap_or(ITEM_NONE);
    let label = format!("Picking up {}", item_name(item));
    let ratio = (pickup_state.elapsed_secs / STRUCTURE_PICKUP_SECONDS).clamp(0.0, 1.0);
    let remaining = (STRUCTURE_PICKUP_SECONDS - pickup_state.elapsed_secs).max(0.0);
    let details = format!("Hold right click: {:.1}s", remaining);

    for mut text in &mut text_queries.p0() {
        *text = Text::new(label.clone());
    }
    for mut node in &mut bar_query {
        node.width = Val::Percent(ratio * 100.0);
    }
    for mut text in &mut text_queries.p1() {
        *text = Text::new(details.clone());
    }
}

pub(crate) fn placement_direction_ui_system(
    placement: Res<PlacementState>,
    mut panel_query: Query<&mut Visibility, With<PlacementDirectionPanel>>,
    mut text_query: Query<&mut Text, With<PlacementDirectionText>>,
) {
    let is_visible = placement.selected == Some(ITEM_INSERTER);
    for mut visibility in &mut panel_query {
        *visibility = if is_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !is_visible {
        return;
    }

    let label = inserter_direction_label(placement.inserter_direction);
    for mut text in &mut text_query {
        *text = Text::new(format!("Inserter output: {label}"));
    }
}

pub(crate) fn inventory_cell_hover_system(
    mut crafting_ui: ResMut<CraftingUiState>,
    mut query: Query<(&Interaction, &InventoryCellButton), Changed<Interaction>>,
) {
    for (interaction, cell) in &mut query {
        match *interaction {
            Interaction::Hovered | Interaction::Pressed => {
                crafting_ui.hovered_item = Some(cell.item);
            }
            Interaction::None if crafting_ui.hovered_item == Some(cell.item) => {
                crafting_ui.hovered_item = None;
            }
            Interaction::None => {}
        }
    }
}

pub(crate) fn crafting_recipe_button_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_mut: ResMut<PlayerState>,
    mut crafting_ui: ResMut<CraftingUiState>,
    mut hotbar: ResMut<HotbarState>,
    mut query: Query<(&Interaction, &RecipeButton), Changed<Interaction>>,
) {
    for (interaction, button) in &mut query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            crafting_ui.focused_recipe = button.recipe_index;
        }

        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(recipe) = recipe_for_index(button.recipe_index) else {
            continue;
        };
        let craft_many = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let mut crafted = 0u32;
        if craft_many {
            while try_craft(&mut player_mut.inventory, recipe) {
                crafted += 1;
                if crafted >= 999 {
                    break;
                }
            }
        } else if try_craft(&mut player_mut.inventory, recipe) {
            crafted = 1;
        }

        if crafted > 0 {
            hotbar.assign_item(recipe.output);
        }
    }
}

pub(crate) fn crafting_panel_text_system(
    player: Res<PlayerState>,
    crafting_ui: Res<CraftingUiState>,
    mut text_queries: ParamSet<(
        Query<(&InventoryItemCountText, &mut Text)>,
        Query<&mut Text, With<RecipeDetailText>>,
    )>,
) {
    for (count, mut text) in &mut text_queries.p0() {
        *text = Text::new(player.inventory.count(count.item).to_string());
    }

    let label = if let Some(item) = crafting_ui.hovered_item {
        item_detail_label(item, &player.inventory)
    } else {
        recipe_for_index(crafting_ui.focused_recipe)
            .map(|recipe| recipe_detail_label(recipe, &player.inventory))
            .unwrap_or_default()
    };
    for mut text in &mut text_queries.p1() {
        *text = Text::new(label.clone());
    }
}

pub(crate) fn crafting_recipe_visual_system(
    player: Res<PlayerState>,
    crafting_ui: Res<CraftingUiState>,
    mut query: Query<(
        &RecipeButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (button, interaction, mut background, mut border) in &mut query {
        let Some(recipe) = recipe_for_index(button.recipe_index) else {
            continue;
        };
        let focused = crafting_ui.focused_recipe == button.recipe_index;
        let can_craft = can_craft(&player.inventory, recipe);
        *background = BackgroundColor(match (focused, *interaction, can_craft) {
            (true, _, true) => Color::srgba(0.14, 0.22, 0.13, 0.86),
            (true, _, false) => Color::srgba(0.2, 0.12, 0.1, 0.86),
            (false, Interaction::Hovered, true) => Color::srgba(0.12, 0.16, 0.11, 0.82),
            (false, Interaction::Hovered, false) => Color::srgba(0.14, 0.1, 0.09, 0.82),
            _ => Color::srgba(0.08, 0.11, 0.09, 0.72),
        });
        *border = BorderColor(if focused {
            Color::srgb(0.98, 0.98, 0.95)
        } else if can_craft {
            Color::srgba(0.76, 0.94, 0.66, 0.4)
        } else {
            Color::srgba(0.94, 0.5, 0.42, 0.32)
        });
    }
}

pub(crate) fn pickup_notice_spawn_system(
    mut commands: Commands,
    icons: Res<UiIconAssets>,
    mut notices: EventReader<PickupNotice>,
    root_query: Query<Entity, With<PickupFeedRoot>>,
    mut toast_query: Query<(&mut PickupNoticeToast, &Children)>,
    mut text_query: Query<&mut Text, With<PickupNoticeText>>,
) {
    let Ok(root) = root_query.single() else {
        return;
    };

    for notice in notices.read() {
        let label = pickup_notice_label(*notice);
        let mut updated_existing = false;
        for (mut toast, children) in &mut toast_query {
            if toast.item != notice.item {
                continue;
            }

            toast.timer = Timer::from_seconds(1.7, TimerMode::Once);
            for child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    *text = Text::new(label.clone());
                    break;
                }
            }
            updated_existing = true;
            break;
        }

        if updated_existing {
            continue;
        }

        commands.entity(root).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(210.0),
                        height: Val::Px(34.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        column_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.055, 0.045, 0.82)),
                    BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                    BorderRadius::all(Val::Px(6.0)),
                    hud_shadow(),
                    PickupNoticeToast {
                        item: notice.item,
                        timer: Timer::from_seconds(1.7, TimerMode::Once),
                    },
                ))
                .with_children(|toast| {
                    toast.spawn((
                        ImageNode::new(icons.for_item(notice.item)),
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    toast.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.96, 0.94)),
                        hud_text_shadow(),
                        PickupNoticeText,
                    ));
                });
        });
    }
}

pub(crate) fn pickup_notice_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut PickupNoticeToast)>,
) {
    for (entity, mut toast) in &mut query {
        toast.timer.tick(time.delta());
        if toast.timer.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn pickup_notice_label(notice: PickupNotice) -> String {
    format!(
        "+{} {} ({})",
        notice.amount,
        item_name(notice.item),
        notice.total
    )
}

pub(crate) fn chest_ui_system(
    ui_state: Res<UiState>,
    runtime: Res<WorldRuntime>,
    player: Res<PlayerState>,
    icons: Res<UiIconAssets>,
    mut slot_buttons: Query<
        (
            &ChestSlotButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<ChestDepositButton>,
    >,
    mut slot_icons: Query<(&ChestSlotIcon, &mut ImageNode), Without<ChestDepositIcon>>,
    mut slot_counts: Query<(&ChestSlotCountText, &mut Text), Without<ChestDepositCountText>>,
    mut deposit_buttons: Query<
        (
            &ChestDepositButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<ChestSlotButton>,
    >,
    mut deposit_icons: Query<(&ChestDepositIcon, &mut ImageNode), Without<ChestSlotIcon>>,
    mut deposit_counts: Query<(&ChestDepositCountText, &mut Text), Without<ChestSlotCountText>>,
) {
    let UiMode::Chest { object_id } = ui_state.mode else {
        return;
    };
    let chest = find_chest(&runtime, object_id);

    for (button, interaction, mut background, mut border) in &mut slot_buttons {
        let slot = chest
            .and_then(|chest| chest.inv.slots.get(button.index))
            .copied();
        let occupied = matches!(slot, Some(slot) if !slot.is_empty());
        let hovered = *interaction == Interaction::Hovered || *interaction == Interaction::Pressed;
        *background = storage_cell_background(occupied, hovered);
        *border = storage_cell_border(occupied);
    }

    for (slot_icon, mut image) in &mut slot_icons {
        let slot = chest
            .and_then(|chest| chest.inv.slots.get(slot_icon.index))
            .copied();
        if let Some(slot) = slot.filter(|slot| !slot.is_empty()) {
            image.image = icons.for_item(slot.item);
            image.color = Color::WHITE;
        } else {
            image.image = icons.empty.clone();
            image.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        }
    }

    for (slot_count, mut text) in &mut slot_counts {
        let count = chest
            .and_then(|chest| chest.inv.slots.get(slot_count.index))
            .filter(|slot| !slot.is_empty())
            .map(|slot| slot.count)
            .unwrap_or(0);
        *text = Text::new(item_count_label(count));
    }

    for (button, interaction, mut background, mut border) in &mut deposit_buttons {
        let count = player.inventory.count(button.item);
        let occupied = count > 0;
        let hovered = *interaction == Interaction::Hovered || *interaction == Interaction::Pressed;
        *background = storage_cell_background(occupied, hovered);
        *border = storage_cell_border(occupied);
    }

    for (deposit_icon, mut image) in &mut deposit_icons {
        image.color = if player.inventory.count(deposit_icon.item) > 0 {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.32)
        };
    }

    for (deposit_count, mut text) in &mut deposit_counts {
        *text = Text::new(item_count_label(player.inventory.count(deposit_count.item)));
    }
}

pub(crate) fn furnace_ui_system(
    ui_state: Res<UiState>,
    runtime: Res<WorldRuntime>,
    player: Res<PlayerState>,
    icons: Res<UiIconAssets>,
    mut slot_buttons: Query<
        (
            &FurnaceSlotButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<FurnaceDepositButton>,
    >,
    mut slot_icons: Query<(&FurnaceSlotIcon, &mut ImageNode), Without<FurnaceDepositIcon>>,
    mut slot_counts: Query<(&FurnaceSlotCountText, &mut Text), Without<FurnaceDepositCountText>>,
    mut deposit_buttons: Query<
        (
            &FurnaceDepositButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<FurnaceSlotButton>,
    >,
    mut deposit_icons: Query<(&FurnaceDepositIcon, &mut ImageNode), Without<FurnaceSlotIcon>>,
    mut deposit_counts: Query<(&FurnaceDepositCountText, &mut Text), Without<FurnaceSlotCountText>>,
    mut bar_query: Query<&mut Node, With<FurnaceProgressBar>>,
) {
    let UiMode::Furnace { object_id } = ui_state.mode else {
        return;
    };
    let furnace = find_furnace(&runtime, object_id);

    for (button, interaction, mut background, mut border) in &mut slot_buttons {
        let slot = furnace_slot(furnace, button.slot);
        let occupied = matches!(slot, Some(slot) if !slot.is_empty());
        let hovered = *interaction == Interaction::Hovered || *interaction == Interaction::Pressed;
        *background = storage_cell_background(occupied, hovered);
        *border = storage_cell_border(occupied);
    }

    for (slot_icon, mut image) in &mut slot_icons {
        if let Some(slot) = furnace_slot(furnace, slot_icon.slot).filter(|slot| !slot.is_empty()) {
            image.image = icons.for_item(slot.item);
            image.color = Color::WHITE;
        } else {
            image.image = icons.empty.clone();
            image.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        }
    }

    for (slot_count, mut text) in &mut slot_counts {
        let count = furnace_slot(furnace, slot_count.slot)
            .filter(|slot| !slot.is_empty())
            .map(|slot| slot.count)
            .unwrap_or(0);
        *text = Text::new(item_count_label(count));
    }

    for (button, interaction, mut background, mut border) in &mut deposit_buttons {
        let count = player.inventory.count(button.item);
        let occupied = count > 0;
        let hovered = *interaction == Interaction::Hovered || *interaction == Interaction::Pressed;
        *background = storage_cell_background(occupied, hovered);
        *border = storage_cell_border(occupied);
    }

    for (deposit_icon, mut image) in &mut deposit_icons {
        image.color = if player.inventory.count(deposit_icon.item) > 0 {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.32)
        };
    }

    for (deposit_count, mut text) in &mut deposit_counts {
        *text = Text::new(item_count_label(player.inventory.count(deposit_count.item)));
    }
    let width = furnace
        .map(|furnace| {
            FURNACE_PROGRESS_BAR_WIDTH
                * (furnace.state.progress as f32 / FURNACE_PROGRESS_PER_ITEM as f32).clamp(0.0, 1.0)
        })
        .unwrap_or(0.0);
    for mut node in &mut bar_query {
        node.width = Val::Px(width);
    }
}

pub(crate) fn inserter_ui_system(
    ui_state: Res<UiState>,
    runtime: Res<WorldRuntime>,
    icons: Res<UiIconAssets>,
    mut slot_buttons: Query<(
        &InserterSlotButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut slot_icons: Query<(&InserterSlotIcon, &mut ImageNode)>,
    mut text_queries: ParamSet<(
        Query<(&InserterSlotCountText, &mut Text)>,
        Query<&mut Text, With<InserterDirectionText>>,
    )>,
) {
    let UiMode::Inserter { object_id } = ui_state.mode else {
        return;
    };
    let inserter = find_inserter(&runtime, object_id);

    let direction_label = inserter
        .map(|inserter| inserter_direction_label(inserter.direction))
        .unwrap_or_else(|| "Missing".to_string());
    for mut text in &mut text_queries.p1() {
        *text = Text::new(format!("Output: {direction_label}"));
    }

    for (button, interaction, mut background, mut border) in &mut slot_buttons {
        let slot = inserter
            .and_then(|inserter| inserter.inv.slots.get(button.index))
            .copied();
        let occupied = matches!(slot, Some(slot) if !slot.is_empty());
        let hovered = *interaction == Interaction::Hovered || *interaction == Interaction::Pressed;
        *background = storage_cell_background(occupied, hovered);
        *border = storage_cell_border(occupied);
    }

    for (slot_icon, mut image) in &mut slot_icons {
        let slot = inserter
            .and_then(|inserter| inserter.inv.slots.get(slot_icon.index))
            .copied();
        if let Some(slot) = slot.filter(|slot| !slot.is_empty()) {
            image.image = icons.for_item(slot.item);
            image.color = Color::WHITE;
        } else {
            image.image = icons.empty.clone();
            image.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        }
    }

    for (slot_count, mut text) in &mut text_queries.p0() {
        let count = inserter
            .and_then(|inserter| inserter.inv.slots.get(slot_count.index))
            .filter(|slot| !slot.is_empty())
            .map(|slot| slot.count)
            .unwrap_or(0);
        *text = Text::new(item_count_label(count));
    }
}

pub(crate) fn storage_status_text_system(
    status: Res<StorageStatus>,
    mut query: Query<&mut Text, With<StorageStatusText>>,
) {
    if !status.is_changed() {
        return;
    }
    let label = status.label();
    for mut text in &mut query {
        *text = Text::new(label.clone());
    }
}

pub(crate) fn world_stats_text_system(
    runtime: Res<WorldRuntime>,
    stats: Res<EvictionStats>,
    mut query: Query<&mut Text, With<WorldStatsText>>,
) {
    if !runtime.is_changed() && !stats.is_changed() {
        return;
    }
    let label = format!(
        "Chunks: {} | Dirty: {} | Evict/s: {}",
        runtime.loaded.len(),
        runtime.dirty.len(),
        stats.evicted_per_second
    );
    for mut text in &mut query {
        *text = Text::new(label.clone());
    }
}
