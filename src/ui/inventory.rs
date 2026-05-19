use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::ui::{UiState, EguiReady};
use crate::player::interaction::Inventory;
use crate::player::combat::WeaponState;
use crate::voxel::BlockType;

/// Draw inventory panel (toggleable with 'I' key)
pub fn draw_inventory_panel(
    mut contexts: EguiContexts,
    ready: Res<EguiReady>,
    mut ui_state: ResMut<UiState>,
    mut inventory: ResMut<Inventory>,
    _weapon: Res<WeaponState>,
    mut placement: ResMut<crate::player::interaction::PlacementState>,
) {
    if !ready.0 || !ui_state.show_inventory {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mouse_pos = ctx.pointer_latest_pos();

    egui::Window::new("inventory_window")
        .anchor(egui::Align2::LEFT_CENTER, [20.0, 0.0])
        .title_bar(false)
        .resizable(false)
        .default_width(380.0)
        .frame(egui::Frame::window(&ctx.style())
            .fill(egui::Color32::from_black_alpha(240))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)))
            .corner_radius(10.0)
            .inner_margin(15.0))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new("⚒ FORGE & INVENTORY")
                        .font(egui::FontId::proportional(22.0))
                        .color(egui::Color32::from_rgb(255, 180, 50))
                        .strong()
                ));
            });
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("📦 Resources");
                ui.add_space(5.0);
                if inventory.resources.is_empty() {
                    ui.label(egui::RichText::new("Mine blocks to collect materials...").italics());
                } else {
                    egui::Grid::new("inventory_grid").spacing([20.0, 8.0]).show(ui, |ui| {
                        let mut items: Vec<_> = inventory.resources.iter().collect();
                        items.sort_by_key(|&(k, _)| format!("{:?}", k));

                        for (block_type, count) in items {
                            if *count == 0 { continue; }

                            let (name, color) = match block_type {
                                BlockType::Wood => ("Wood", egui::Color32::from_rgb(139, 69, 19)),
                                BlockType::Stone => ("Stone", egui::Color32::GRAY),
                                BlockType::IronOre => ("Iron Ore", egui::Color32::from_rgb(180, 160, 150)),
                                BlockType::GoldOre => ("Gold Ore", egui::Color32::from_rgb(255, 215, 0)),
                                BlockType::CraftString => ("String", egui::Color32::from_rgb(220, 220, 200)),
                                BlockType::Gear => ("Gear", egui::Color32::from_rgb(150, 150, 180)),
                                BlockType::Axle => ("Axle", egui::Color32::from_rgb(140, 140, 140)),
                                _ => ("Resource", egui::Color32::WHITE),
                            };

                            ui.horizontal(|ui| {
                                ui.colored_label(color, "■");
                                ui.label(name);
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(count.to_string()).strong());
                            });
                            ui.end_row();
                        }
                    });
                }
            });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                // --- TOOLS ---
                draw_manual_collapsing(ui, ctx, mouse_pos, "⛏ Tiered Tools", |ui| {
                    ui.add_space(5.0);
                    // Pickaxes
                    draw_tier_row(ui, &mut inventory, "Wooden Pickaxe", "Basic stone mining",
                        &[(BlockType::Wood, 3), (BlockType::Stone, 2)],
                        |inv| inv.has_pickaxe, |inv| inv.has_pickaxe = true, mouse_pos, ctx);

                    draw_tier_row(ui, &mut inventory, "Iron Pickaxe", "2x Mining Speed",
                        &[(BlockType::Wood, 2), (BlockType::IronOre, 5)],
                        |inv| inv.has_iron_pickaxe, |inv| inv.has_iron_pickaxe = true, mouse_pos, ctx);

                    draw_tier_row(ui, &mut inventory, "Gold Pickaxe", "4x Mining Speed",
                        &[(BlockType::IronOre, 5), (BlockType::GoldOre, 5)],
                        |inv| inv.has_gold_pickaxe, |inv| inv.has_gold_pickaxe = true, mouse_pos, ctx);

                    ui.separator();

                    // Axes
                    draw_tier_row(ui, &mut inventory, "Basic Axe", "Fell trees faster",
                        &[(BlockType::Wood, 4)],
                        |inv| inv.has_axe, |inv| inv.has_axe = true, mouse_pos, ctx);

                    draw_tier_row(ui, &mut inventory, "Iron Axe", "2x Wood speed",
                        &[(BlockType::IronOre, 4)],
                        |inv| inv.has_iron_axe, |inv| inv.has_iron_axe = true, mouse_pos, ctx);
                });

                // --- COMBAT ---
                draw_manual_collapsing(ui, ctx, mouse_pos, "⚔ Combat Equipment", |ui| {
                    ui.add_space(5.0);
                    draw_tier_row(ui, &mut inventory, "Iron Sword", "Basic defense",
                        &[(BlockType::Wood, 1), (BlockType::IronOre, 3)],
                        |inv| inv.has_sword, |inv| inv.has_sword = true, mouse_pos, ctx);

                    draw_tier_row(ui, &mut inventory, "Gold Sword", "High damage",
                        &[(BlockType::IronOre, 2), (BlockType::GoldOre, 5)],
                        |inv| inv.has_gold_sword, |inv| inv.has_gold_sword = true, mouse_pos, ctx);

                    draw_tier_row(ui, &mut inventory, "Hunter Bow", "Ranged (Wood = Arrows)",
                        &[(BlockType::Wood, 5), (BlockType::CraftString, 4)],
                        |inv| inv.has_bow, |inv| inv.has_bow = true, mouse_pos, ctx);
                });

                // --- MECH ---
                draw_manual_collapsing(ui, ctx, mouse_pos, "🦾 Mech Upgrades", |ui| {
                    ui.add_space(5.0);
                    use crate::player::interaction::ArmorTier;

                    draw_tier_row(ui, &mut inventory, "Iron Plating", "+25% Protection",
                        &[(BlockType::IronOre, 15)],
                        |inv| matches!(inv.armor_tier, ArmorTier::Iron | ArmorTier::Gold),
                        |inv| inv.armor_tier = ArmorTier::Iron, mouse_pos, ctx);

                    draw_tier_row(ui, &mut inventory, "Gold Plating", "+50% Protection",
                        &[(BlockType::GoldOre, 15)],
                        |inv| matches!(inv.armor_tier, ArmorTier::Gold),
                        |inv| inv.armor_tier = ArmorTier::Gold, mouse_pos, ctx);
                });
                
                // --- MARITIME ---
                draw_manual_collapsing(ui, ctx, mouse_pos, "⛵ Maritime", |ui| {
                    ui.add_space(5.0);
                    draw_tier_row(ui, &mut inventory, "Wooden Boat", "Float on rivers",
                        &[(BlockType::Wood, 5)],
                        |inv| inv.resources.get(&BlockType::Boat).copied().unwrap_or(0) > 0,
                        |inv| { *inv.resources.entry(BlockType::Boat).or_insert(0) += 1; },
                        mouse_pos, ctx);
                });

                // --- CONSTRUCTION ---
                draw_manual_collapsing(ui, ctx, mouse_pos, "🧱 Construction & Architecture", |ui| {
                    ui.add_space(5.0);
                    egui::Grid::new("block_selector_grid").spacing([10.0, 10.0]).show(ui, |ui| {
                        let blocks = [
                            (BlockType::Stone, "Stone", Color::from(bevy::color::palettes::css::GRAY)),
                            (BlockType::Brick, "Brick", Color::from(bevy::color::palettes::css::ORANGE_RED)),
                            (BlockType::Concrete, "Concrete", Color::from(bevy::color::palettes::css::DARK_GRAY)),
                            (BlockType::IronBlock, "Iron", Color::from(bevy::color::palettes::css::SILVER)),
                            (BlockType::WoodPlanks, "Wood", Color::from(bevy::color::palettes::css::BROWN)),
                            (BlockType::Glass, "Glass", Color::from(bevy::color::palettes::css::LIGHT_BLUE)),
                            (BlockType::Slope, "Slope", Color::from(bevy::color::palettes::css::BROWN)),
                            (BlockType::SlopeCorner, "Corner", Color::from(bevy::color::palettes::css::BROWN)),
                            (BlockType::SlopeValley, "Valley", Color::from(bevy::color::palettes::css::BROWN)),
                            (BlockType::Door, "Door", Color::from(bevy::color::palettes::css::SANDY_BROWN)),
                            (BlockType::CastleDoor, "Castle Door", Color::from(bevy::color::palettes::css::DARK_GRAY)),
                            (BlockType::ProceduralWall, "Procedural Wall", Color::from(bevy::color::palettes::css::GOLD)),
                        ];

                        for (i, (block, name, _color)) in blocks.iter().enumerate() {
                            let is_selected = placement.current_block == *block;
                            let btn = ui.add(egui::Button::new(*name).stroke(if is_selected { 
                                egui::Stroke::new(2.0, egui::Color32::YELLOW) 
                            } else { 
                                egui::Stroke::NONE 
                            }));
                            
                            if if let Some(pos) = mouse_pos {
                                btn.rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                            } else {
                                false
                            } {
                                placement.current_block = *block;
                            }

                            if (i + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                });

                // --- MACHINERY ---
                draw_manual_collapsing(ui, ctx, mouse_pos, "⚙ Industrial Machinery", |ui| {
                    ui.add_space(5.0);
                    egui::Grid::new("machinery_selector_grid").spacing([10.0, 10.0]).show(ui, |ui| {
                        let machines = [
                            (BlockType::Generator, "Generator"),
                            (BlockType::Motor, "Motor"),
                            (BlockType::Gear, "Gear"),
                            (BlockType::Axle, "Axle"),
                        ];

                        for (i, (block, name)) in machines.iter().enumerate() {
                            let is_selected = placement.current_block == *block;
                            let btn = ui.add(egui::Button::new(*name).stroke(if is_selected { 
                                egui::Stroke::new(2.0, egui::Color32::GOLD) 
                            } else { 
                                egui::Stroke::NONE 
                            }));
                            
                            if if let Some(pos) = mouse_pos {
                                btn.rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                            } else {
                                false
                            } {
                                placement.current_block = *block;
                            }

                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });

            ui.add_space(20.0);
            ui.vertical_centered_justified(|ui| {
                let close_btn = ui.button(egui::RichText::new("CLOSE").strong());
                let close_rect = close_btn.rect;
                let close_clicked = if let Some(pos) = mouse_pos {
                    close_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                } else {
                    false
                };
                if close_clicked {
                    ui_state.show_inventory = false;
                }
            });
        });
}

fn draw_tier_row(
    ui: &mut egui::Ui,
    inventory: &mut Inventory,
    name: &str,
    desc: &str,
    ingredients: &[(BlockType, u32)],
    check_crafted: impl Fn(&Inventory) -> bool,
    on_craft: impl FnOnce(&mut Inventory),
    mouse_pos: Option<egui::Pos2>,
    ctx: &egui::Context,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(name).strong());
                ui.add(egui::Label::new(egui::RichText::new(desc).size(10.0)).wrap_mode(egui::TextWrapMode::Extend));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if check_crafted(inventory) {
                    ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "✔ OWNED");
                } else {
                    let mut can_afford = true;
                    let mut tooltip = "Requires:".to_string();
                    for (block, count) in ingredients {
                        let has = inventory.resources.get(block).copied().unwrap_or(0);
                        if has < *count { can_afford = false; }
                        tooltip.push_str(&format!("\n • {:?}: {}/{}", block, has, count));
                    }

                    let btn = ui.add_enabled(can_afford, egui::Button::new("CRAFT"));
                    let btn_rect = btn.rect;

                    // Manual click detection for macOS workaround
                    let button_clicked = if can_afford {
                        if let Some(pos) = mouse_pos {
                            btn_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if button_clicked {
                        for (block, count) in ingredients {
                            *inventory.resources.entry(*block).or_insert(0) -= count;
                        }
                        on_craft(inventory);
                        info!("Crafted {}", name);
                    }
                    btn.on_hover_text(tooltip);
                }
            });
        });
    });
    ui.add_space(4.0);
}

fn draw_manual_collapsing(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    mouse_pos: Option<egui::Pos2>,
    name: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.make_persistent_id(name);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, false);

    let _header_res = ui.horizontal(|ui| {
        let text = if state.is_open() { "▼ " } else { "▶ " }.to_string() + name;
        let btn = ui.add(egui::Button::new(egui::RichText::new(text).strong().size(16.0)).frame(false));
        
        let header_clicked = if let Some(pos) = mouse_pos {
            btn.rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
        } else {
            false
        };

        if header_clicked {
            state.toggle(ui);
        }
    });

    state.show_body_unindented(ui, |ui| {
        ui.indent(id.with("body"), |ui| {
            add_contents(ui);
        });
    });
    ui.add_space(5.0);
}
