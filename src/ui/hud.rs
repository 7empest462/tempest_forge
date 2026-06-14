use crate::player::camera::{CameraMode, MechSuit, PhysicsState, Player};
use crate::player::combat::{AmmoState, LaserHeat, WeaponState};
use crate::ui::EguiReady;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// Draw HUD overlay - always visible on screen
pub fn draw_hud(
    mut contexts: EguiContexts,
    ready: Res<EguiReady>,
    ui_state: Res<crate::ui::UiState>,
    player_query: Query<(&PhysicsState, &CameraMode, &MechSuit), With<Player>>,
    weapon: Res<WeaponState>,
    laser_heat: Res<LaserHeat>,
    ammo_state: Res<AmmoState>,
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
                            ui.label(
                                egui::RichText::new("MECH SUIT")
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 180, 50)),
                            );
                            let status_color = if mech.active {
                                egui::Color32::GREEN
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(
                                status_color,
                                if mech.active {
                                    "● ACTIVE"
                                } else {
                                    "○ STANDBY"
                                },
                            );
                        });
                        ui.add_space(5.0);

                        ui.label(format!(
                            "Modules: Jump Lvl {}, Mining Lvl {}",
                            mech.jump_level, mech.mining_level
                        ));
                        ui.label(format!("Mode: {:?}", mode));

                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label("Velocity:");
                            ui.add(
                                egui::ProgressBar::new((physics.speed / 20.0).min(1.0))
                                    .text(format!("{:.1} m/s", physics.speed))
                                    .desired_width(120.0),
                            );
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
                            WeaponState::Pistol => "PISTOL",
                            WeaponState::Revolver => "REVOLVER",
                            WeaponState::Rifle => "ASSAULT RIFLE",
                            WeaponState::Sniper => "SNIPER RIFLE",
                        };
                        ui.label(egui::RichText::new(format!("WEAPON: {}", weapon_label)).strong());

                        if matches!(
                            *weapon,
                            WeaponState::Pistol
                                | WeaponState::Revolver
                                | WeaponState::Rifle
                                | WeaponState::Sniper
                        ) {
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                let (current_ammo, max_ammo) = match *weapon {
                                    WeaponState::Pistol => (ammo_state.pistol_ammo, 12),
                                    WeaponState::Revolver => (ammo_state.revolver_ammo, 6),
                                    WeaponState::Rifle => (ammo_state.rifle_ammo, 30),
                                    WeaponState::Sniper => (ammo_state.sniper_ammo, 5),
                                    _ => (0, 0),
                                };
                                ui.label("AMMO:");
                                if let Some(reloading_wp) = ammo_state.reloading_weapon {
                                    if reloading_wp == *weapon {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 200, 50),
                                            "RELOADING...",
                                        );
                                    } else {
                                        ui.label(format!("{}/{}", current_ammo, max_ammo));
                                    }
                                } else {
                                    ui.label(format!("{}/{}", current_ammo, max_ammo));
                                }
                            });
                        }

                        if *weapon == WeaponState::Laser {
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.label("THERMAL:");
                                let mut bar = egui::ProgressBar::new(laser_heat.current / 100.0)
                                    .desired_width(120.0);
                                if laser_heat.overheated {
                                    bar = bar.text(
                                        egui::RichText::new("OVERHEATED").color(egui::Color32::RED),
                                    );
                                } else {
                                    bar = bar.text(format!("{:.0}%", laser_heat.current));
                                }
                                ui.add(bar);
                            });
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("BUILDING:");
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 255, 100),
                                format!("{:?}", placement.current_block),
                            );
                        });

                        if placement.current_block == crate::voxel::BlockType::ProceduralWall {
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new("PROCEDURAL WALL:")
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 215, 0)),
                            );
                            match ui_state.input_scheme {
                                crate::ui::InputScheme::KeyboardMouse => {
                                    ui.label("• Right-Click: Place Point");
                                    ui.label("• Backspace: Undo Last Point");
                                    ui.label("• Escape: Cancel Wall");
                                    ui.label("• Arrow Up/Down: Adjust Height");
                                    ui.label("• Enter: Build Wall!");
                                }
                                crate::ui::InputScheme::SteamDeck => {
                                    ui.label("• LT (L2): Place Point");
                                    ui.label("• LB (L1): Undo Last Point");
                                    ui.label("• B Button: Cancel Wall");
                                    ui.label("• D-Pad Left/Right: Adjust Height");
                                    ui.label("• RT (R2): Build Wall!");
                                }
                            }
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
                ui.vertical(|ui| match ui_state.input_scheme {
                    crate::ui::InputScheme::KeyboardMouse => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("WASD").strong());
                            ui.label("Move |");
                            ui.label(egui::RichText::new("SHIFT").strong());
                            ui.label("Sprint |");
                            ui.label(egui::RichText::new("SPACE/CTRL").strong());
                            ui.label("Fly Up/Down |");
                            ui.label(egui::RichText::new("I/E").strong());
                            ui.label("Inventory |");
                            ui.label(egui::RichText::new("M").strong());
                            ui.label("Toggle Mech |");
                            ui.label(egui::RichText::new("1-3").strong());
                            ui.label("Tools |");
                            ui.label(egui::RichText::new("Z-B").strong());
                            ui.label("Build Blocks |");
                            ui.label(egui::RichText::new("Right-Click").strong());
                            ui.label("Place Block |");
                            ui.label(egui::RichText::new("F5/F9").strong());
                            ui.label("Save/Load |");
                            ui.label(egui::RichText::new("ESC").strong());
                            ui.label("Pause");
                        });
                    }
                    crate::ui::InputScheme::SteamDeck => {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("L-STICK")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Move |");
                            ui.label(
                                egui::RichText::new("R-STICK")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Look |");
                            ui.label(
                                egui::RichText::new("L3")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Sprint |");
                            ui.label(
                                egui::RichText::new("A")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Jump |");
                            ui.label(
                                egui::RichText::new("B")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Dive |");
                            ui.label(
                                egui::RichText::new("X")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Fly |");
                            ui.label(
                                egui::RichText::new("Y")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Mech Suit |");
                            ui.label(
                                egui::RichText::new("D-PAD L/U/R")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Equip Tools |");
                            ui.label(
                                egui::RichText::new("D-PAD ↓")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Inventory |");
                            ui.label(
                                egui::RichText::new("RT")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Mine/Action |");
                            ui.label(
                                egui::RichText::new("LT")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Place |");
                            ui.label(
                                egui::RichText::new("SELECT")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Camera |");
                            ui.label(
                                egui::RichText::new("START")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.label("Menu");
                        });
                    }
                });
            });
        });

    // Center Screen Crosshair
    if !ui_state.show_inventory && !ui_state.show_pause_menu {
        egui::Area::new(egui::Id::new("crosshair_area"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let center = rect.center();
                let painter = ui.painter();
                let color = egui::Color32::from_rgb(0, 255, 255); // neon cyan
                let stroke = egui::Stroke::new(1.5, color);

                // Draw horizontal tick marks
                painter.line_segment(
                    [
                        center - egui::vec2(12.0, 0.0),
                        center - egui::vec2(4.0, 0.0),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        center + egui::vec2(4.0, 0.0),
                        center + egui::vec2(12.0, 0.0),
                    ],
                    stroke,
                );
                // Draw vertical tick marks
                painter.line_segment(
                    [
                        center - egui::vec2(0.0, 12.0),
                        center - egui::vec2(0.0, 4.0),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        center + egui::vec2(0.0, 4.0),
                        center + egui::vec2(0.0, 12.0),
                    ],
                    stroke,
                );

                // Central aiming point
                painter.circle_filled(center, 1.5, color);
            });
    }
}
