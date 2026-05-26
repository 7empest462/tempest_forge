use bevy::{prelude::*, window::CursorOptions};
use bevy::window::{PrimaryWindow, CursorGrabMode};
use crate::GameState;
use bevy_voxel_world::prelude::*;

pub mod hud;
pub mod inventory;
pub mod pause_menu;

/// Control scheme modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputScheme {
    KeyboardMouse,
    SteamDeck,
}

impl Default for InputScheme {
    fn default() -> Self {
        #[cfg(target_os = "linux")]
        {
            InputScheme::SteamDeck
        }
        #[cfg(not(target_os = "linux"))]
        {
            InputScheme::KeyboardMouse
        }
    }
}

/// UI state tracking which panels are open
#[derive(Resource, Default, PartialEq, Eq)]
pub struct UiState {
    pub show_inventory: bool,
    pub show_pause_menu: bool,
    pub input_scheme: InputScheme,
}

/// Resource to track if egui is ready for drawing (fonts initialized)
#[derive(Resource, Default, PartialEq, Eq)]
pub struct EguiReady(pub bool);

/// UI Plugin - manages all egui-based UI panels
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<EguiReady>()
            .init_resource::<pause_menu::PauseMenuState>()
            .add_systems(Update, egui_warmup)
            // Loading Screen Systems
            .add_systems(Update, draw_loading_screen
                .run_if(in_state(GameState::Loading))
                .run_if(resource_equals(EguiReady(true))))
            // In-Game Systems
            .add_systems(Update, (
                ui_input_handler,
                hud::draw_hud.after(egui_warmup),
                inventory::draw_inventory_panel.after(hud::draw_hud),
                pause_menu::draw_pause_menu.after(inventory::draw_inventory_panel),
                grab_mouse.after(pause_menu::draw_pause_menu),
            ).run_if(in_state(GameState::InGame)));
    }
}

/// System to draw the loading screen while chunks are generating
fn draw_loading_screen(
    mut contexts: bevy_egui::EguiContexts,
    chunk_query: Query<Entity, With<Chunk<crate::world::noise_generator::NoiseGenerator>>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer += time.delta_secs();
    let chunk_count = chunk_query.iter().count();
    let target_chunks = 128; // Increased for visibility
    let progress = (chunk_count as f32 / target_chunks as f32).min(1.0);

    let Ok(ctx) = contexts.ctx_mut() else { return; };
    bevy_egui::egui::CentralPanel::default()
        .frame(bevy_egui::egui::Frame::NONE.fill(bevy_egui::egui::Color32::from_rgb(10, 10, 15)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.3);
                ui.heading(bevy_egui::egui::RichText::new("TEMPEST FORGE")
                    .size(60.0)
                    .color(bevy_egui::egui::Color32::from_rgb(0, 200, 255))
                    .strong());
                
                ui.add_space(40.0);
                ui.label(bevy_egui::egui::RichText::new("Forging the world...")
                    .size(24.0)
                    .italics());

                ui.add_space(20.0);
                let progress_bar = bevy_egui::egui::ProgressBar::new(progress)
                    .show_percentage()
                    .animate(true);
                ui.add(progress_bar);

                ui.add_space(10.0);
                ui.label(format!("Chunks Generated: {} / {}", chunk_count, target_chunks));
                
                if *timer < 2.0 {
                    ui.add_space(10.0);
                    ui.label(bevy_egui::egui::RichText::new("Warming up engines...")
                        .color(bevy_egui::egui::Color32::GRAY));
                }
            });
        });

    if chunk_count >= target_chunks && *timer >= 2.0 {
        next_state.set(GameState::InGame);
    }
}

/// Handle UI input (toggle panels with keys + quick save/load)
fn ui_input_handler(
    input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut ui_state: ResMut<UiState>,
    mut save_events: MessageWriter<crate::persistence::SaveEvent>,
    mut load_events: MessageWriter<crate::persistence::LoadEvent>,
    gamepads: Query<&Gamepad>,
) {
    let mut toggle_inv = input.just_pressed(KeyCode::KeyE);
    let mut toggle_pause = input.just_pressed(KeyCode::Escape);

    // Check gamepad input FIRST — Steam Deck HID emits both keyboard and gamepad
    // events simultaneously, so we must detect gamepad before the keyboard fallback.
    let mut any_gamepad_activity = false;
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::Start) { toggle_pause = true; }
        if gamepad.just_pressed(GamepadButton::DPadDown) { toggle_inv = true; }

        // Check if ANY gamepad button was pressed this frame
        let gp_pressed = gamepad.just_pressed(GamepadButton::Start)
            || gamepad.just_pressed(GamepadButton::DPadDown)
            || gamepad.just_pressed(GamepadButton::South)
            || gamepad.just_pressed(GamepadButton::East)
            || gamepad.just_pressed(GamepadButton::North)
            || gamepad.just_pressed(GamepadButton::West)
            || gamepad.just_pressed(GamepadButton::LeftTrigger2)
            || gamepad.just_pressed(GamepadButton::RightTrigger2)
            || gamepad.just_pressed(GamepadButton::LeftTrigger)
            || gamepad.just_pressed(GamepadButton::RightTrigger)
            || gamepad.just_pressed(GamepadButton::Select)
            || gamepad.just_pressed(GamepadButton::LeftThumb)
            || gamepad.just_pressed(GamepadButton::RightThumb)
            || gamepad.just_pressed(GamepadButton::DPadLeft)
            || gamepad.just_pressed(GamepadButton::DPadUp)
            || gamepad.just_pressed(GamepadButton::DPadRight);

        // Also detect analog stick movement as gamepad activity
        let lx = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let ly = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
        let rx = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
        let ry = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
        let stick_active = lx.abs() > 0.15 || ly.abs() > 0.15 || rx.abs() > 0.15 || ry.abs() > 0.15;

        if gp_pressed || stick_active {
            any_gamepad_activity = true;
            if ui_state.input_scheme != InputScheme::SteamDeck {
                ui_state.input_scheme = InputScheme::SteamDeck;
                info!("Gamepad input detected. Control scheme switched to Steam Deck.");
            }
        }
    }

    // Only switch to Keyboard & Mouse if there was NO gamepad activity this frame.
    // This prevents the Steam Deck's dual keyboard+gamepad HID events from
    // constantly bouncing the scheme back to keyboard.
    if !any_gamepad_activity {
        if input.get_just_pressed().next().is_some() || mouse_input.get_just_pressed().next().is_some() {
            if ui_state.input_scheme != InputScheme::KeyboardMouse {
                ui_state.input_scheme = InputScheme::KeyboardMouse;
                info!("Keyboard/Mouse input detected. Control scheme switched to Keyboard & Mouse.");
            }
        }
    }

    if toggle_inv {
        ui_state.show_inventory = !ui_state.show_inventory;
    }

    if toggle_pause {
        ui_state.show_pause_menu = !ui_state.show_pause_menu;
    }

    // Quick Save F5
    if input.just_pressed(KeyCode::F5) {
        save_events.write(crate::persistence::SaveEvent);
    }

    // Quick Load F9
    if input.just_pressed(KeyCode::F9) {
        load_events.write(crate::persistence::LoadEvent);
    }
}

/// System to handle egui warm-up delay to prevent font panics
pub fn egui_warmup(
    mut ready: ResMut<EguiReady>,
    mut frame_count: Local<u32>,
) {
    if !ready.0 {
        *frame_count += 1;
        if *frame_count > 10 {
            ready.0 = true;
        }
    }
}

/// System to release/grab cursor based on UI state and Egui interaction
fn grab_mouse(
    mut windows: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    ui_state: Res<UiState>,
    mut contexts: bevy_egui::EguiContexts,
) {
    let Ok((_, mut cursor_options)) = windows.single_mut() else { return; };

    // ALWAYS release cursor when ANY menu is open
    if ui_state.show_pause_menu || ui_state.show_inventory {
        if cursor_options.grab_mode != CursorGrabMode::None {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
        return;
    }

    // Check if egui wants input
    let egui_wants_input = contexts.ctx_mut()
        .map(|ctx: &mut bevy_egui::egui::Context| ctx.is_pointer_over_area() || ctx.wants_pointer_input() || ctx.wants_keyboard_input())
        .unwrap_or(false);

    if egui_wants_input {
        if cursor_options.grab_mode != CursorGrabMode::None {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    } else {
        // Grab cursor during gameplay (no menus, no egui focus)
        if cursor_options.grab_mode != CursorGrabMode::Locked {
            cursor_options.grab_mode = CursorGrabMode::Locked;
            cursor_options.visible = false;
        }
    }
}

