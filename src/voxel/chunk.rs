use bevy::prelude::*;
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
#[repr(u8)]
pub enum BlockType {
    #[default]
    Air,
    Water,
    // Liquids / semi-liquids
    Lava,
    Oil,

    // Ground / Soil variants
    Sand,
    Gravel,
    Clay,
    Mud,
    Grass,
    Dirt,
    Loam,   // or fertile soil
    Podzol, // forest floor variant

    // Stone / Geological (real rock diversity)
    Stone, // generic or bedrock
    Cobblestone,
    Basalt,
    Limestone,
    Granite,
    Slate,
    Sandstone,
    Marble,
    Obsidian,
    Andesite,
    Diorite,
    Gabbro, // or other igneous
    Shale,
    Chalk,

    // Ores & Minerals
    IronOre,
    GoldOre,
    Coal,
    CopperOre,
    TinOre, // for bronze age feel
    DiamondOre,
    Quartz,
    // Add more like Silver, Lead, etc. as needed

    // Processed / Construction Materials (key for real ratios)
    Brick, // standard clay brick (e.g., ~215x102.5x65mm)
    StoneBrick,
    Concrete,
    ReinforcedConcrete,
    Glass, // or GlassPane as separate if thin
    Plaster,
    Mortar,  // for binding
    Asphalt, // roads
    CeramicTile,

    // Wood & Organic (real lumber emphasis)
    Wood,       // log
    WoodPlanks, // generic or add OakPlanks, PinePlanks, etc.
    // For real ratios: consider variants like TwoByFour, TwoBySix, Beam, etc.
    // or use associated data later for orientation/size
    OakLog,
    PineLog,
    Bamboo,
    Leaves,
    Fern,
    Flower,
    Hay, // or Straw
    Moss,

    // Metals & Processed
    IronBlock,
    SteelBlock,
    GoldBlock,
    CopperBlock,

    // Mechanical / Functional (expand your existing set)
    Generator,
    Motor,
    Gear,
    Axle,
    Pipe,
    Conveyor,
    Furnace,
    Crafter, // or Workbench
    Chest,
    Boat,

    // Misc / Decorative
    Snow,
    Ice,
    CraftString, // Retained from original

    // Architectural / Interactive
    Door,
    CastleDoor,
    SlidingDoor,
    Slope,
    SlopeCorner,
    SlopeValley,
    ProceduralWall,

    // Alien Blocks
    AlienStone,
    AlienDirt,
    GlowingMoss,
    AlienCrystal,
    FloatingCrystal,
}

impl From<u8> for BlockType {
    fn from(value: u8) -> Self {
        // Use a safe transmute-like mapping or a match
        // Because the enum has [repr(u8)] or default repr, we can map it
        // A robust match is better
        unsafe { std::mem::transmute(value.min(BlockType::FloatingCrystal as u8)) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Drill,
    Laser,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockProperties {
    pub name: &'static str,        // for UI/debug
    pub voxel_size: IVec3,         // Real-life ratio in voxels (e.g. brick = 4x2x1)
    pub density: f32,              // kg/m³-ish, for Rapier mass
    pub compressive_strength: f32, // arbitrary units for destruction
    pub is_transparent: bool,
    pub is_vegetation: bool,
    pub flammable: bool,
    pub harvest_tool: Option<ToolType>, // enum you define
    pub base_hardness: f32,             // time to mine
}

pub static DEFAULT_PROPS: BlockProperties = BlockProperties {
    name: "Unknown",
    voxel_size: IVec3::ONE,
    density: 1000.0,
    compressive_strength: 10.0,
    is_transparent: false,
    is_vegetation: false,
    flammable: false,
    harvest_tool: None,
    base_hardness: 1.0,
};

use rustc_hash::FxHashMap;

#[derive(Resource)]
pub struct BlockRegistry {
    pub props: FxHashMap<BlockType, BlockProperties>,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        build_block_registry()
    }
}

pub fn build_block_registry() -> BlockRegistry {
    let mut registry = BlockRegistry {
        props: FxHashMap::with_capacity_and_hasher(80, Default::default()),
    };

    // === TERRAIN ===
    registry.props.insert(
        BlockType::Stone,
        BlockProperties {
            name: "Stone",
            voxel_size: IVec3::ONE,
            density: 2600.0,
            compressive_strength: 120.0,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::Dirt,
        BlockProperties {
            name: "Dirt",
            voxel_size: IVec3::ONE,
            density: 1400.0,
            compressive_strength: 5.0,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::Grass,
        BlockProperties {
            name: "Grass",
            voxel_size: IVec3::ONE,
            is_vegetation: true,
            ..DEFAULT_PROPS
        },
    );

    // === CONSTRUCTION - REAL RATIOS ===
    registry.props.insert(
        BlockType::Brick,
        BlockProperties {
            name: "Clay Brick",
            voxel_size: IVec3::new(4, 2, 1), // Example: if 1 voxel = 5cm → ~20x10x5cm (close to real UK brick)
            density: 1900.0,
            compressive_strength: 25.0,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::WoodPlanks,
        BlockProperties {
            name: "Wood Planks",
            voxel_size: IVec3::new(1, 1, 4), // e.g. 2x4 equivalent
            density: 550.0,
            flammable: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::Wood,
        BlockProperties {
            name: "Wood Log",
            voxel_size: IVec3::new(2, 2, 8), // longer in one axis
            density: 650.0,
            flammable: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::Concrete,
        BlockProperties {
            name: "Concrete",
            voxel_size: IVec3::new(8, 4, 2), // standard concrete block size
            density: 2400.0,
            compressive_strength: 40.0,
            ..DEFAULT_PROPS
        },
    );

    // === VEGETATION ===
    registry.props.insert(
        BlockType::Leaves,
        BlockProperties {
            name: "Leaves",
            voxel_size: IVec3::ONE,
            is_vegetation: true,
            flammable: true,
            ..DEFAULT_PROPS
        },
    );

    // === LIQUIDS ===
    registry.props.insert(
        BlockType::Water,
        BlockProperties {
            name: "Water",
            voxel_size: IVec3::ONE,
            density: 1000.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::Glass,
        BlockProperties {
            name: "Glass",
            voxel_size: IVec3::ONE,
            density: 2500.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::Slope,
        BlockProperties {
            name: "Roof Slope",
            voxel_size: IVec3::ONE,
            density: 600.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::SlopeCorner,
        BlockProperties {
            name: "Roof Corner",
            voxel_size: IVec3::ONE,
            density: 600.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::SlopeValley,
        BlockProperties {
            name: "Roof Valley",
            voxel_size: IVec3::ONE,
            density: 600.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::ProceduralWall,
        BlockProperties {
            name: "Procedural Wall",
            voxel_size: IVec3::ONE,
            density: 1500.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );
    // ========================================
    // === ALIEN WORLD BLOCKS ===
    // ========================================

    registry.props.insert(
        BlockType::AlienStone,
        BlockProperties {
            name: "Alien Stone",
            voxel_size: IVec3::ONE,
            density: 2800.0,
            compressive_strength: 80.0,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::AlienDirt,
        BlockProperties {
            name: "Alien Soil",
            voxel_size: IVec3::ONE,
            density: 1600.0,
            compressive_strength: 8.0,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::GlowingMoss,
        BlockProperties {
            name: "Glowing Moss",
            voxel_size: IVec3::ONE,
            density: 400.0,
            is_vegetation: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::AlienCrystal,
        BlockProperties {
            name: "Alien Crystal",
            voxel_size: IVec3::ONE,
            density: 3200.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry.props.insert(
        BlockType::FloatingCrystal,
        BlockProperties {
            name: "Floating Crystal",
            voxel_size: IVec3::ONE,
            density: 1800.0,
            is_transparent: true,
            ..DEFAULT_PROPS
        },
    );

    registry
}

// ========================
// ALIEN MATERIALS
// ========================

#[derive(Resource)]
pub struct AlienMaterials {
    pub alien_stone: Handle<StandardMaterial>,
    pub alien_dirt: Handle<StandardMaterial>,
    pub glowing_moss: Handle<StandardMaterial>,
    pub alien_crystal: Handle<StandardMaterial>,
    pub floating_crystal: Handle<StandardMaterial>,
}

pub fn build_alien_block_materials(materials: &mut Assets<StandardMaterial>) -> AlienMaterials {
    AlienMaterials {
        alien_stone: materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.25, 0.48),
            ..default()
        }),

        alien_dirt: materials.add(StandardMaterial {
            base_color: Color::srgb(0.38, 0.28, 0.42),
            ..default()
        }),

        glowing_moss: materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.75, 0.35),
            emissive: LinearRgba::new(0.8, 2.8, 1.6, 1.0), // Correct type
            ..default()
        }),

        alien_crystal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.85, 1.0),
            emissive: LinearRgba::new(1.6, 3.6, 6.4, 1.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),

        floating_crystal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.55, 1.0),
            emissive: LinearRgba::new(4.4, 2.4, 7.6, 1.0),
            ..default()
        }),
    }
}
