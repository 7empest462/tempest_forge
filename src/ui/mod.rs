use crate::GameState;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy::{prelude::*, window::CursorOptions};
use bevy_voxel_world::prelude::*;

pub mod hud;
pub mod inventory;
pub mod pause_menu;

/// Control scheme modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputScheme {
    #[default]
    KeyboardMouse,
    SteamDeck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InventoryCategory {
    #[default]
    Resources,
    Tools,
    Combat,
    Mech,
    Build,
    Machinery,
    Maritime,
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
            // Main Menu Screen Systems
            .add_systems(
                Update,
                (
                    draw_main_menu.run_if(resource_equals(EguiReady(true))),
                    debug_menu_input.run_if(resource_equals(EguiReady(true))),
                    enforce_menu_cursor,
                )
                    .run_if(in_state(GameState::MainMenu)),
            )
            // Loading Screen Systems
            .add_systems(
                Update,
                draw_loading_screen
                    .run_if(in_state(GameState::Loading))
                    .run_if(resource_equals(EguiReady(true))),
            )
            // In-Game Systems
            .add_systems(
                Update,
                (
                    ui_input_handler,
                    hud::draw_hud.after(egui_warmup),
                    inventory::draw_inventory_panel.after(hud::draw_hud),
                    pause_menu::draw_pause_menu.after(inventory::draw_inventory_panel),
                    grab_mouse.after(pause_menu::draw_pause_menu),
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

/// System to draw the loading screen while chunks are generating
fn draw_loading_screen(
    mut contexts: bevy_egui::EguiContexts,
    chunk_query: Query<Entity, With<Chunk<crate::world::noise_generator::NoiseGenerator>>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut timer: Local<f32>,
    time: Res<Time>,
    _voxel_world: bevy_voxel_world::prelude::VoxelWorld<
        crate::world::noise_generator::NoiseGenerator,
    >,
    _player_query: Query<&Transform, With<crate::player::camera::Player>>,
) {
    *timer += time.delta_secs();
    let chunk_count = chunk_query.iter().count();
    let target_chunks = 128; // Increased for visibility
    let progress = (chunk_count as f32 / target_chunks as f32).min(1.0);

    let ctx = match contexts.ctx_mut() {
        Ok(c) => c,
        Err(e) => {
            warn!("[DEBUG] draw_loading_screen: egui context error: {:?}", e);
            return;
        }
    };

    bevy_egui::egui::CentralPanel::default()
        .frame(bevy_egui::egui::Frame::NONE.fill(bevy_egui::egui::Color32::from_rgb(10, 10, 15)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.3);
                ui.heading(
                    bevy_egui::egui::RichText::new("TEMPEST FORGE")
                        .size(60.0)
                        .color(bevy_egui::egui::Color32::from_rgb(0, 200, 255))
                        .strong(),
                );

                ui.add_space(40.0);
                ui.label(
                    bevy_egui::egui::RichText::new("Forging the world...")
                        .size(24.0)
                        .italics(),
                );

                ui.add_space(20.0);
                let progress_bar = bevy_egui::egui::ProgressBar::new(progress)
                    .show_percentage()
                    .animate(true);
                ui.add(progress_bar);

                ui.add_space(10.0);
                ui.label(format!(
                    "Chunks Generated: {} / {}",
                    chunk_count, target_chunks
                ));

                if *timer < 2.0 {
                    ui.add_space(10.0);
                    ui.label(
                        bevy_egui::egui::RichText::new("Warming up engines...")
                            .color(bevy_egui::egui::Color32::GRAY),
                    );
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
        if gamepad.just_pressed(GamepadButton::Start) {
            toggle_pause = true;
        }
        if gamepad.just_pressed(GamepadButton::DPadDown) {
            toggle_inv = true;
        }

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
    if !any_gamepad_activity
        && (input.get_just_pressed().next().is_some()
            || mouse_input.get_just_pressed().next().is_some())
        && ui_state.input_scheme != InputScheme::KeyboardMouse
    {
        ui_state.input_scheme = InputScheme::KeyboardMouse;
        info!("Keyboard/Mouse input detected. Control scheme switched to Keyboard & Mouse.");
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
pub fn egui_warmup(mut ready: ResMut<EguiReady>, mut frame_count: Local<u32>) {
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
    let Ok((_, mut cursor_options)) = windows.single_mut() else {
        return;
    };

    // ALWAYS release cursor when ANY menu is open
    if ui_state.show_pause_menu || ui_state.show_inventory {
        if cursor_options.grab_mode != CursorGrabMode::None {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
        return;
    }

    // Check if egui wants input
    let egui_wants_input = contexts
        .ctx_mut()
        .map(|ctx: &mut bevy_egui::egui::Context| {
            ctx.is_pointer_over_area() || ctx.wants_pointer_input() || ctx.wants_keyboard_input()
        })
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

/// System to draw the main menu screen
fn draw_main_menu(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
    mut frame_counter: Local<u32>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut was_play_hovered: Local<bool>,
    mut was_exit_hovered: Local<bool>,
) {
    *frame_counter += 1;
    if *frame_counter % 60 == 0 {
        info!("[DEBUG MENU] draw_main_menu runs. Frame count: {}", *frame_counter);
        if let Ok(window) = windows.single() {
            info!(
                "[DEBUG SCALE] Window size logical: {}x{}, physical: {}x{}, scale factor: {}",
                window.width(), window.height(),
                window.physical_width(), window.physical_height(),
                window.scale_factor()
            );
        }
    }

    let ctx = match contexts.ctx_mut() {
        Ok(c) => c,
        Err(e) => {
            if *frame_counter % 60 == 0 {
                warn!("[DEBUG MENU] ctx_mut returned error: {:?}", e);
            }
            return;
        }
    };

    if *frame_counter % 60 == 0 {
        info!(
            "[DEBUG SCALE] Egui screen rect: {:?}",
            ctx.screen_rect()
        );
        info!("[DEBUG MENU] Egui context retrieved successfully. Pointer pos: {:?}", ctx.pointer_latest_pos());
    }

    ctx.input(|i| {
        info!(
            "[DEBUG EGUI INPUT] Pointer pos: {:?}, pressed: {}, released: {}, buttons: {:?}, events: {:?}",
            i.pointer.latest_pos(),
            i.pointer.any_pressed(),
            i.pointer.any_released(),
            i.pointer.button_down(bevy_egui::egui::PointerButton::Primary),
            i.events
        );
    });

    bevy_egui::egui::CentralPanel::default()
        .frame(bevy_egui::egui::Frame::NONE.fill(bevy_egui::egui::Color32::from_black_alpha(100)))
        .show(ctx, |ui| {


            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);

                // Title
                ui.heading(
                    bevy_egui::egui::RichText::new("TEMPEST FORGE")
                        .size(72.0)
                        .color(bevy_egui::egui::Color32::from_rgb(0, 200, 255))
                        .strong(),
                );

                ui.add_space(10.0);
                ui.label(
                    bevy_egui::egui::RichText::new("Procedural Voxel & Water Simulation")
                        .size(20.0)
                        .color(bevy_egui::egui::Color32::from_rgb(200, 240, 255))
                        .italics(),
                );

                ui.add_space(60.0);

                // Menu Buttons
                let button_width = 250.0;
                let button_height = 40.0;

                ui.scope(|ui| {
                    ui.style_mut().spacing.item_spacing = bevy_egui::egui::vec2(0.0, 15.0);

                    let play_btn = ui.add_sized(
                        [button_width, button_height],
                        bevy_egui::egui::Button::new(
                            bevy_egui::egui::RichText::new("🚀 FORGE NEW WORLD")
                                .size(18.0)
                                .strong()
                                .color(bevy_egui::egui::Color32::WHITE),
                        )
                        .fill(bevy_egui::egui::Color32::from_rgb(0, 120, 200)),
                    );

                    let play_hovered = play_btn.hovered();
                    if play_hovered != *was_play_hovered {
                        *was_play_hovered = play_hovered;
                        info!("[DEBUG MENU] FORGE NEW WORLD button hover changed: {} (Pointer: {:?})", play_hovered, ctx.pointer_latest_pos());
                    }

                    info!(
                        "[DEBUG EGUI FRAME] Frame: {}, pointer_pos: {:?}, play_rect: {:?}, hovered: {}, clicked: {}, focused: {}, pointer_down: {}",
                        *frame_counter,
                        ctx.pointer_latest_pos(),
                        play_btn.rect,
                        play_btn.hovered(),
                        play_btn.clicked(),
                        ctx.input(|i| i.focused),
                        ctx.input(|i| i.pointer.any_down())
                    );
                    info!(
                        "[DEBUG EGUI HIT] ui.clip_rect(): {:?}, ui.rect_contains_pointer(): {}, ui.available_rect_before_wrap(): {:?}",
                        ui.clip_rect(),
                        ui.rect_contains_pointer(play_btn.rect),
                        ui.available_rect_before_wrap()
                    );
                    let is_over = ctx.is_pointer_over_area();
                    let wants_input = ctx.wants_pointer_input();
                    let has_ptr = ctx.input(|i| i.pointer.has_pointer());
                    let hover_pos = ctx.input(|i| i.pointer.hover_pos());
                    let dragged_id = ctx.dragged_id();
                    let is_dragging = dragged_id.is_some();
                    ctx.memory(|mem| {
                        info!(
                            "[DEBUG EGUI MEM] focused: {:?}, ui.is_enabled(): {}, is_pointer_over_area(): {}, wants_pointer_input(): {}, has_pointer: {}, hover_pos: {:?}, dragged_id: {:?}, dragging: {}",
                            mem.focused(),
                            ui.is_enabled(),
                            is_over,
                            wants_input,
                            has_ptr,
                            hover_pos,
                            dragged_id,
                            is_dragging
                        );
                    });
                    ctx.viewport(|vp| {
                        info!("[DEBUG WIDGETS THIS PASS] count of layers: {}", vp.this_pass.widgets.layers().count());
                        for (layer_id, rects) in vp.this_pass.widgets.layers() {
                            info!("  Layer: {:?}", layer_id);
                            for w in rects {
                                info!("    Widget ID: {:?}, rect: {:?}", w.id, w.rect);
                            }
                        }
                    });
                    ctx.viewport(|vp| {
                        info!("[DEBUG WIDGETS PREV PASS] count of layers: {}", vp.prev_pass.widgets.layers().count());
                        for (layer_id, rects) in vp.prev_pass.widgets.layers() {
                            info!("  Layer: {:?}", layer_id);
                            for w in rects {
                                info!("    Widget ID: {:?}, rect: {:?}", w.id, w.rect);
                            }
                        }
                    });
                    info!("[DEBUG PLAY RESPONSE] {:?}", play_btn);
                    ctx.input(|i| {
                        info!(
                            "[DEBUG EGUI PTR] press_origin: {:?}, delta: {:?}",
                            i.pointer.press_origin(),
                            i.pointer.delta()
                        );
                    });

                    if play_btn.clicked() {
                        info!("[DEBUG MENU] FORGE NEW WORLD button clicked! Setting state to Loading.");
                        next_state.set(GameState::Loading);
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let exit_btn = ui.add_sized(
                            [button_width, button_height],
                            bevy_egui::egui::Button::new(
                                bevy_egui::egui::RichText::new("🚪 EXIT GAME")
                                    .size(18.0)
                                    .strong()
                                    .color(bevy_egui::egui::Color32::WHITE),
                            )
                            .fill(bevy_egui::egui::Color32::from_rgb(180, 50, 50)),
                        );

                        let exit_hovered = exit_btn.hovered();
                        if exit_hovered != *was_exit_hovered {
                            *was_exit_hovered = exit_hovered;
                            info!("[DEBUG MENU] EXIT GAME button hover changed: {} (Pointer: {:?})", exit_hovered, ctx.pointer_latest_pos());
                        }

                        info!(
                            "[DEBUG EGUI FRAME EXIT] exit_rect: {:?}, hovered: {}, clicked: {}",
                            exit_btn.rect,
                            exit_btn.hovered(),
                            exit_btn.clicked()
                        );

                        if exit_btn.clicked() {
                            info!("[DEBUG MENU] EXIT GAME button clicked! Exiting.");
                            std::process::exit(0);
                        }
                    }
                });
            });
        });
}

/// System to print mouse clicks and cursor positions in MainMenu for debugging
fn debug_menu_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    ready: Res<EguiReady>,
) {
    if ready.0 && mouse_input.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            info!(
                "[DEBUG INPUT] Mouse left click! Window cursor pos: {:?}",
                window.cursor_position()
            );
        }
    }
}

/// System to ensure the cursor is always unlocked and visible during the Main Menu
fn enforce_menu_cursor(
    mut windows: Query<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
) {
    if let Ok(mut cursor_options) = windows.single_mut() {
        if cursor_options.grab_mode != bevy::window::CursorGrabMode::None {
            info!("[DEBUG CURSOR] Main menu cursor was grabbed! Unlocking it now.");
            cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
        }
        if !cursor_options.visible {
            info!("[DEBUG CURSOR] Main menu cursor was hidden! Making it visible now.");
            cursor_options.visible = true;
        }
    }
}
