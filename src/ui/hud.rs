use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::ui::EguiReady;
use crate::player::camera::{Player, PhysicsState, CameraMode, MechSuit};
use crate::player::combat::{WeaponState, LaserHeat};

/// Draw HUD overlay - always visible on screen
pub fn draw_hud(
    mut contexts: EguiContexts,
    ready: Res<EguiReady>,
    ui_state: Res<crate::ui::UiState>,
    player_query: Query<(&PhysicsState, &CameraMode, &MechSuit), With<Player>>,
    weapon: Res<WeaponState>,
    laser_heat: Res<LaserHeat>,
    placement: Res<crate::player::interaction::PlacementState>,
) {
    if !ready.0 {
        return;
    }
    let Ok((physics, mode, mech)) = player_query.single() else {
        return;
    };

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let hud_frame = egui::Frame::default()
        .fill(egui::Color32::from_black_alpha(150))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
        .corner_radius(5.0)
        .inner_margin(10.0);

    // Top-left: Mech Status (only if inventory is closed)
    if !ui_state.show_inventory {
        egui::Area::new(egui::Id::new("mech_status_area"))
            .anchor(egui::Align2::LEFT_TOP, [15.0, 15.0])
            .show(ctx, |ui| {
                hud_frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("MECH SUIT").strong().color(egui::Color32::from_rgb(255, 180, 50)));
                            let status_color = if mech.active { egui::Color32::GREEN } else { egui::Color32::RED };
                            ui.colored_label(status_color, if mech.active { "● ACTIVE" } else { "○ STANDBY" });
                        });
                        ui.add_space(5.0);
                        
                        ui.label(format!("Modules: Jump Lvl {}, Mining Lvl {}", mech.jump_level, mech.mining_level));
                        ui.label(format!("Mode: {:?}", mode));
                        
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label("Velocity:");
                            ui.add(egui::ProgressBar::new((physics.speed / 20.0).min(1.0))
                                .text(format!("{:.1} m/s", physics.speed))
                                .desired_width(120.0));
                        });

                        let (state_text, state_color) = if physics.flying {
                            ("FLYING", egui::Color32::from_rgb(100, 180, 255))
                        } else if physics.grounded {
                            ("GROUNDED", egui::Color32::GREEN)
                        } else {
                            ("FALLING", egui::Color32::YELLOW)
                        };
                        ui.colored_label(state_color, format!("STATE: {}", state_text));

                        ui.separator();
                        let weapon_label = match *weapon {
                            WeaponState::NoWeapon => "HANDS",
                            WeaponState::Pickaxe => "PICKAXE",
                            WeaponState::Axe => "AXE",
                            WeaponState::Sword => "SWORD",
                            WeaponState::Bow => "BOW",
                            WeaponState::Laser => "ION LASER",
                        };
                        ui.label(egui::RichText::new(format!("WEAPON: {}", weapon_label)).strong());

                        if *weapon == WeaponState::Laser {
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.label("THERMAL:");
                                let mut bar = egui::ProgressBar::new(laser_heat.current / 100.0).desired_width(120.0);
                                if laser_heat.overheated {
                                    bar = bar.text(egui::RichText::new("OVERHEATED").color(egui::Color32::RED));
                                } else {
                                    bar = bar.text(format!("{:.0}%", laser_heat.current));
                                }
                                ui.add(bar);
                            });
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("BUILDING:");
                            ui.colored_label(egui::Color32::from_rgb(100, 255, 100), format!("{:?}", placement.current_block));
                        });

                        if placement.current_block == crate::voxel::BlockType::ProceduralWall {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("PROCEDURAL WALL:").strong().color(egui::Color32::from_rgb(255, 215, 0)));
                            ui.label("• Right-Click: Place Point");
                            ui.label("• Backspace: Undo Last Point");
                            ui.label("• Escape: Cancel Wall");
                            ui.label("• Enter/Return: Build Wall!");
                        }
                    });
                });
            });
    }

    // Bottom-center: Controls Hint (Compact)
    egui::Area::new(egui::Id::new("controls_area"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -20.0])
        .show(ctx, |ui| {
            hud_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("WASD").strong()); ui.label("Move |");
                    ui.label(egui::RichText::new("SHIFT").strong()); ui.label("Sprint |");
                    ui.label(egui::RichText::new("SPACE").strong()); ui.label("Fly Up |");
                    ui.label(egui::RichText::new("I").strong()); ui.label("Inventory |");
                    ui.label(egui::RichText::new("M").strong()); ui.label("Toggle Mech |");
                    ui.label(egui::RichText::new("1-5").strong()); ui.label("Weapons |");
                    ui.label(egui::RichText::new("Z-B").strong()); ui.label("Build Blocks |");
                    ui.label(egui::RichText::new("Right-Click").strong()); ui.label("Place Block |");
                    ui.label(egui::RichText::new("F5/F9").strong()); ui.label("Save/Load");
                });
            });
        });
}
