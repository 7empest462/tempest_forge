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

            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Game Controls");
                    ui.separator();

                    ui.label("WASD - Move forward/left/back/right");
                    ui.label("SPACE - Jump");
                    ui.label("SHIFT - Sprint");
                    ui.label("Left Click - Mine blocks");
                    ui.label("Right Click - Place blocks");
                    ui.label("I - Toggle Inventory");
                    ui.label("ESC - Toggle Pause Menu");
                    ui.label("C - Switch Camera Mode");
                    ui.label("F - Toggle Flight");
                });
            });

            ui.separator();
            ui.label("Current Frame Time:");
            ui.label("(Performance monitoring coming soon)");

            ui.separator();

            // Get mouse position for manual click detection
            let mouse_pos = ctx.pointer_latest_pos();

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
                let resume_btn = ui.button("Resume (ESC)");
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

