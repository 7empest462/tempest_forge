use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::ui::{UiState, EguiReady};
use crate::persistence::{SaveEvent, LoadEvent};

#[derive(Resource, Default)]
pub struct PauseMenuState;

/// Draw pause menu (toggleable with ESC key)
pub fn draw_pause_menu(
    mut contexts: EguiContexts,
    ready: Res<EguiReady>,
    mut ui_state: ResMut<UiState>,
    mut save_events: MessageWriter<SaveEvent>,
    mut load_events: MessageWriter<LoadEvent>,
) {
    if !ready.0 || !ui_state.show_pause_menu {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("⏸ Pause Menu")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("PAUSED");
            ui.separator();

            // Get mouse position for manual click detection
            let mouse_pos = ctx.pointer_latest_pos();

            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Game Controls");
                    ui.separator();

                    match ui_state.input_scheme {
                        crate::ui::InputScheme::KeyboardMouse => {
                            ui.label("WASD - Move Player");
                            ui.label("Mouse - Look Around");
                            ui.label("Shift - Sprint");
                            ui.label("Space - Fly Up / Jump");
                            ui.label("Control - Fly Down / Dive");
                            ui.label("I / E - Open Inventory & Crafting");
                            ui.label("M - Toggle Mech Suit");
                            ui.label("1, 2, 3 - Equip Mech Tools (Drill, Axe, Laser)");
                            ui.label("Z, X, C, V, B - Select Build Block Type");
                            ui.label("Left Click - Mine / Attack");
                            ui.label("Right Click - Place Block");
                            ui.label("F5 / F9 - Quick Save / Load");
                            ui.label("Escape - Toggle Pause Menu");
                        }
                        crate::ui::InputScheme::SteamDeck => {
                            ui.label("L-Stick - Move  |  R-Stick - Look");
                            ui.label("A Button - Jump / Swim Up");
                            ui.label("B Button - Crouch / Swim Down");
                            ui.label("L3 - Sprint");
                            ui.label("R2 (RT) - Mine Blocks / Attack");
                            ui.label("L2 (LT) - Place Blocks");
                            ui.label("D-Pad Down - Toggle Inventory");
                            ui.label("Start - Toggle Pause Menu");
                            ui.label("Select - Switch Camera Mode");
                            ui.label("X Button - Toggle Flight Mode");
                            ui.label("Y Button - Toggle Mech Suit");
                            ui.label("D-Pad Left/Up/Right - Equip Tools");
                        }
                    }
                });
            });

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Control Scheme:").strong());
                let scheme_btn = ui.button(format!("🔄 Switch to {}", match ui_state.input_scheme {
                    crate::ui::InputScheme::KeyboardMouse => "Steam Deck / Gamepad",
                    crate::ui::InputScheme::SteamDeck => "Keyboard & Mouse",
                }));
                let scheme_rect = scheme_btn.rect;
                let scheme_clicked = if let Some(pos) = mouse_pos {
                    scheme_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                } else {
                    false
                };
                if scheme_clicked {
                    ui_state.input_scheme = match ui_state.input_scheme {
                        crate::ui::InputScheme::KeyboardMouse => crate::ui::InputScheme::SteamDeck,
                        crate::ui::InputScheme::SteamDeck => crate::ui::InputScheme::KeyboardMouse,
                    };
                    info!("Control scheme manually switched via Pause Menu.");
                }
            });

            ui.separator();
            ui.label("Current Frame Time:");
            ui.label("(Performance monitoring coming soon)");

            ui.separator();

            ui.horizontal(|ui| {
                let save_btn = ui.button("Save Game");
                let save_rect = save_btn.rect;
                let save_clicked = if let Some(pos) = mouse_pos {
                    save_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                } else {
                    false
                };

                if save_clicked {
                    save_events.write(SaveEvent);
                }

                let load_btn = ui.button("Load Game");
                let load_rect = load_btn.rect;
                let load_clicked = if let Some(pos) = mouse_pos {
                    load_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                } else {
                    false
                };

                if load_clicked {
                    info!("Load button clicked!");
                    load_events.write(LoadEvent);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                let resume_label = match ui_state.input_scheme {
                    crate::ui::InputScheme::KeyboardMouse => "Resume (ESC)",
                    crate::ui::InputScheme::SteamDeck => "Resume (START)",
                };
                let resume_btn = ui.button(resume_label);
                let resume_rect = resume_btn.rect;
                let resume_clicked = if let Some(pos) = mouse_pos {
                    resume_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                } else {
                    false
                };

                if resume_clicked {
                    ui_state.show_pause_menu = false;
                }

                let quit_btn = ui.button("Quit to Desktop");
                let quit_rect = quit_btn.rect;
                let quit_clicked = if let Some(pos) = mouse_pos {
                    quit_rect.contains(pos) && ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary))
                } else {
                    false
                };

                if quit_clicked {
                    std::process::exit(0);
                }
            });

            ui.separator();
            ui.label("Version: Tempest Forge 0.1.0");
        });
}

