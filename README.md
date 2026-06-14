# Tempest Forge

**A cyber-voxel sandbox where creation meets chaos.**

Pilot a powerful **flying mech suit** and reshape entire worlds. Build intricate structures with procedural generation, carve terrain with a devastating laser cannon, and watch realistic water systems dynamically flow into the landscapes you create.

![Tempest Forge Screenshot](assets/screenshot1.png) <!-- Replace with actual screenshots later -->
![Flying Mech](assets/mech_flight.gif)

> [!NOTE]
> **Development Note:** This project was developed offline for a long time before being published to GitHub. The current commit count does not fully reflect the total development effort.

---

## Features

### Core Gameplay & Mech Suit
- **Power Mech Suit** — Step inside a devastating mech suit with expressive, arm-based flight animations and massive dual thrusters. Toggle between walking and soaring flight seamlessly.
- **Auto-Step Climbing** — Walk up single-block height changes smoothly without needing to jump (acting like stair stepping). Taller obstructions require manual jumps, keeping traversal fast and natural.
- **Ion Laser Cannon** — A high-powered terraforming and mining beam that clears land rapidly. Equipped with active overheating thermal management (up to 6.7 seconds of continuous firing with a 4.0-second cooling phase).
- **Viscous Water Physics** — Fully integrated fluid simulation. Carve terrain to watch water naturally pool, stream, and flow. Swim, dive, tread water, or pilot a wooden boat with custom buoyancy and treading physics.

### Ranged Combat & Firearms
- **Four Distinct Firearms** — Equip and fire the **Pistol**, **Revolver**, **Rifle**, and **Sniper Rifle**, each with custom damage, rate of fire, ammo capacity, and reload durations.
- **Direct Raycast Aiming** — Crosshairs dynamically map via a camera-forward raycast to their 3D voxel intersection point in both first-person and third-person perspectives, ensuring pixel-perfect accuracy.
- **Tactile Weapon & Camera Recoil** — Screen-shake recoil triggers on shots and decays exponentially over time, paired with physically animated weapon model translation offsets (slide kick-back, lift, and barrel tilt-up rotation).
- **High-Fidelity Audio Synthesis** — Custom, realistic mechanical reloading slides, hammer clicks, and metallic chamber ringing synced with AI-synthesized gunshot audio.

### AI Monsters & Settlements
- **Night Surface Spawning** — Monsters including the elephant-sized quadrupedal **Cyclops** and the bipedal hopping **Triangaroo** emerge underground and during night cycles, automatically despawning in daylight surface zones.
- **Guard Patrols & Settlement Defense** — City Guards actively patrol and engage nearby monsters within 16.0 meters to protect the town, while townspeople flee or retaliate against attackers.

### Procedural Architecture & Physics
- **Procedural Wall Generation** — Lay down anchor points to sketch walls. Once built, individual stone bricks physically drop, bounce, and settle into place via **Rapier3D** physics.
- **Dynamic Voussoir Arches** — Build structures near one another, and watch organic, structurally sound voussoir arches procedurally spawn between nearby wall sections.
- **Dynamic Gateway Carving** — Look at any procedural brick wall and carve custom doorways/gateways dynamically with a simple keystroke!

### Engine & Survival Systems
- **Tiered Tools & Combat** — Craft and upgrade tools (Pickaxes, Axes, Swords, Bows) across Wood, Iron, and Gold tiers. Upgrade your mech suit plating to boost protection.
- **Structured Error Handling** — Built on the `thiserror` crate to gracefully intercept and log I/O, JSON, and entity query errors rather than crashing the game client.
- **Hanabi GPU Particles** — Visually stunning thruster exhausts, laser impact embers, and mining debris.
- **World Persistence** — Full quick-save and quick-load serialization to persist your voxels, structures, inventory, and player state.
- **WASM + WebGPU Support** — Built to run directly in modern browsers with WebGPU hardware acceleration, as well as native desktop builds.

---

## Controls

`Tempest Forge` features dual input schemes tailored for both desktop play and handhelds like the **Steam Deck**.

### Keyboard & Mouse

| Binding | Action |
|---|---|
| **W, A, S, D** | Move |
| **Shift** | Sprint |
| **Space** | Jump / Swim Up / Fly Up |
| **Left Ctrl** | Dive / Descend / Fly Down |
| **F** | Toggle Thrusters / Jetpack Flight |
| **M** | Toggle Mech Suit (Active / Standby) |
| **V** | Cycle Camera View (First Person / Third Person / Front View) |
| **1 - 9** | Equip Tools/Weapons (*Pickaxe, Axe, Sword, Laser, Bow, Pistol, Revolver, Rifle, Sniper*) |
| **Left Click** | Use Tool / Attack / Fire Weapon / Mine |
| **Right Click** | Place Standard Voxel |
| **R** | Manual Reload (when carrying a gun) |
| **I / E** | Toggle Forge & Inventory Menu |
| **Escape** | Toggle Pause Menu / Release Cursor |
| **F5** | Quick Save |
| **F9** | Quick Load |

#### Keyboard: Procedural Wall Construction
*Activate **Procedural Wall** in the construction inventory panel first.*
- **Right-Click**: Place anchor point (draws wall path preview).
- **Backspace**: Undo last anchor point.
- **Escape**: Cancel current wall path.
- **Arrow Up / Arrow Down**: Increase / decrease wall height layers.
- **Enter**: Build wall (triggers physics-based brick drop!).
- **G Key**: Press while looking at a brick wall to dynamically carve a gateway/doorway.

---

### Gamepad & Steam Deck

| Button | Action |
|---|---|
| **Left Stick** | Move |
| **Right Stick** | Look / Aim |
| **L3 (Left Stick Click)** | Sprint |
| **A (South Button)** | Jump / Swim Up / Fly Up |
| **B (East Button)** | Dive / Descend / Fly Down |
| **X (West Button)** | Toggle Thrusters / Jetpack Flight / Manual Reload (when carrying a gun) |
| **Y (North Button)** | Toggle Mech Suit (Active / Standby) |
| **R1 (Right Bumper)** | Cycle Equip Tools/Weapons Forward |
| **L1 (Left Bumper)** | Cycle Equip Tools/Weapons Backward |
| **R2 (Right Trigger)** | Use Tool / Attack / Fire Weapon / Mine |
| **L2 (Left Trigger)** | Place Standard Voxel |
| **D-Pad Left / Up / Right**| Quick Equip Tools (*Drill, Axe, Laser*) |
| **D-Pad Down** | Toggle Forge & Inventory Menu |
| **Select** | Cycle Camera View |
| **Start** | Toggle Pause Menu |

#### Gamepad: Procedural Wall Construction
*Activate **Procedural Wall** in the construction inventory panel first.*
- **Left Trigger (L2)**: Place anchor point.
- **Left Bumper (L1)**: Undo last anchor point.
- **B Button (East)**: Cancel current wall path.
- **D-Pad Left / D-Pad Right**: Decrease / increase wall height layers.
- **Right Trigger (R2)**: Build wall (triggers physics-based brick drop!).

---

## Tech Stack

- **Language**: Rust
- **Engine**: Bevy 0.18 (Entity Component System)
- **Physics**: Rapier3D (colliders, rigid bodies, characters, fluid buoyancy)
- **Voxels**: Custom high-performance rendering powered by `bevy_voxel_world`
- **Particles**: GPU-accelerated particle effects via `bevy_hanabi`
- **UI**: Sleek, glassmorphic layout via `bevy_egui`
- **Targets**: Native Desktop + Web (WebGPU & WASM)

---

## How to Run

### Native Desktop
Ensure you have the Rust toolchain installed. Run:
```bash
cargo run --release
```

### Web (WASM + WebGPU)
To compile and serve the project for the web using modern WebGPU graphics APIs:
1. Install `wasm-bindgen-cli`:
   ```bash
   cargo install wasm-bindgen-cli
   ```
2. Build the WASM target:
   ```bash
   cargo build --release --target wasm32-unknown-unknown
   ```
3. Bind and package:
   ```bash
   wasm-bindgen --out-dir out --target web target/wasm32-unknown-unknown/release/tempest_forge.wasm
   ```
4. Serve the `out` directory using any local web server (e.g. `python3 -m http.server 8080`) and open it in a WebGPU-compatible browser!

---

## License

Licensed under the **Tempest Forge Source-Available License 1.0**. See the [LICENSE](file:///Volumes/Corsair_Lab/Home/Projects/tempest_forge/LICENSE) file for the full text.
