pub mod water {
    use super::water_gpu::{
        WATER_GRID_SIZE, WaterComputePlugin, WaterGpuHandles, WaterSimParams,
    };
    use crate::player::camera::{CameraPivot, PhysicsState, Player};
    use crate::player::combat::Health;
    use crate::world::noise_generator::NoiseGenerator;
    use bevy::asset::RenderAssetUsages;
    use bevy::camera::{RenderTarget, visibility::RenderLayers};
    use bevy::mesh::Indices;
    use bevy::mesh::VertexAttributeValues;
    use bevy::prelude::*;
    use bevy::render::render_resource::*;
    use bevy::render::storage::ShaderStorageBuffer;
    use bevy::shader::ShaderRef;
    use bevy_voxel_world::prelude::*;
    use rand::RngExt;
    pub struct WaterPlugin;
    pub struct MainCamera;
    impl ::bevy::ecs::component::Component for MainCamera
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    pub struct ReflectionCamera;
    impl ::bevy::ecs::component::Component for ReflectionCamera
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    pub struct Dripping {
        pub timer: Timer,
    }
    impl ::bevy::ecs::component::Component for Dripping
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    struct SafeInsertDripping {
        entity: Entity,
        dripping: Dripping,
    }
    impl Command for SafeInsertDripping {
        fn apply(self, world: &mut World) {
            if let Ok(mut entity_mut) = world.get_entity_mut(self.entity) {
                entity_mut.insert(self.dripping);
            }
        }
    }
    pub struct ReflectionTarget {
        pub image: Handle<Image>,
    }
    impl ::bevy::ecs::resource::Resource for ReflectionTarget
    where
        Self: Send + Sync + 'static,
    {}
    impl Plugin for WaterPlugin {
        fn build(&self, app: &mut App) {
            app.add_message::<WaterImpulseEvent>();
            app.add_plugins((
                    MaterialPlugin::<WaterMaterial>::default(),
                    WaterComputePlugin,
                ))
                .add_systems(OnEnter(crate::GameState::InGame), setup_water)
                .add_systems(
                    Update,
                    (
                        (
                            center_water_on_player,
                            update_water_wall_mask,
                            entity_water_interaction,
                            update_dripping,
                        ),
                        (
                            handle_water_l_key,
                            process_water_impulses,
                            apply_buoyancy,
                            boat_ai,
                        ),
                    )
                        .run_if(in_state(crate::GameState::InGame)),
                )
                .add_systems(
                    Update,
                    (sync_reflection_camera, update_water_material)
                        .after(crate::player::camera::player_move)
                        .after(crate::player::camera::player_look)
                        .run_if(in_state(crate::GameState::InGame)),
                )
                .add_systems(OnExit(crate::GameState::InGame), cleanup_water);
        }
    }
    pub struct WaterImpulseEvent {
        pub position: Vec3,
        pub force: f32,
        pub radius: f32,
    }
    impl ::bevy::ecs::message::Message for WaterImpulseEvent
    where
        Self: Send + Sync + 'static,
    {}
    pub struct Boat;
    impl ::bevy::ecs::component::Component for Boat
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    pub struct Buoyant {
        pub force: f32,
    }
    impl ::bevy::ecs::component::Component for Buoyant
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    pub struct WaterMesh {
        pub handle: Handle<Mesh>,
    }
    impl ::bevy::ecs::component::Component for WaterMesh
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    pub struct WaterSimData {
        pub height: Vec<f32>,
        pub flow_x: Vec<f32>,
        pub flow_y: Vec<f32>,
        pub wall_mask: Vec<bool>,
        pub last_disturbed_pos: Option<(usize, usize)>,
        pub size: f32,
        pub grid_len: usize,
        pub dirty: bool,
    }
    impl ::bevy::ecs::component::Component for WaterSimData
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    impl WaterSimData {
        pub fn new(grid_len: usize, size: f32) -> Self {
            let count = grid_len * grid_len;
            let mut sim = Self {
                height: ::alloc::vec::from_elem(1.0, count),
                flow_x: ::alloc::vec::from_elem(0.0, count),
                flow_y: ::alloc::vec::from_elem(0.0, count),
                wall_mask: ::alloc::vec::from_elem(false, count),
                last_disturbed_pos: None,
                size,
                grid_len,
                dirty: true,
            };
            for i in 0..grid_len {
                sim.set_wall(i, 0, true);
                sim.set_wall(i, grid_len - 1, true);
                sim.set_wall(0, i, true);
                sim.set_wall(grid_len - 1, i, true);
            }
            sim
        }
        #[inline]
        pub fn idx(&self, x: usize, y: usize) -> usize {
            x * self.grid_len + y
        }
        #[inline]
        pub fn get_height(&self, x: usize, y: usize) -> f32 {
            self.height[self.idx(x, y)]
        }
        #[inline]
        pub fn set_height(&mut self, x: usize, y: usize, val: f32) {
            let idx = self.idx(x, y);
            self.height[idx] = val;
        }
        #[inline]
        pub fn get_flow_x(&self, x: usize, y: usize) -> f32 {
            self.flow_x[self.idx(x, y)]
        }
        #[inline]
        pub fn set_flow_x(&mut self, x: usize, y: usize, val: f32) {
            let idx = self.idx(x, y);
            self.flow_x[idx] = val;
        }
        #[inline]
        pub fn get_flow_y(&self, x: usize, y: usize) -> f32 {
            self.flow_y[self.idx(x, y)]
        }
        #[inline]
        pub fn set_flow_y(&mut self, x: usize, y: usize, val: f32) {
            let idx = self.idx(x, y);
            self.flow_y[idx] = val;
        }
        #[inline]
        pub fn is_wall(&self, x: usize, y: usize) -> bool {
            self.wall_mask[self.idx(x, y)]
        }
        #[inline]
        pub fn set_wall(&mut self, x: usize, y: usize, val: bool) {
            let idx = self.idx(x, y);
            self.wall_mask[idx] = val;
        }
    }
    pub struct WaterMaterial {
        #[uniform(0)]
        pub color: Vec4,
        #[uniform(0)]
        pub time: f32,
        #[uniform(0)]
        pub camera_position: Vec3,
        #[uniform(0)]
        pub resolution: Vec2,
        #[uniform(0)]
        pub water_level: f32,
        #[uniform(0)]
        pub grid_scale: f32,
        #[uniform(0)]
        pub cloudiness: f32,
        #[texture(1)]
        #[sampler(2)]
        pub reflection_texture: Option<Handle<Image>>,
        #[storage(3, read_only, visibility(vertex, fragment))]
        pub height_buffer: Handle<ShaderStorageBuffer>,
    }
    impl ::bevy::asset::Asset for WaterMaterial {}
    impl ::bevy::asset::VisitAssetDependencies for WaterMaterial {
        fn visit_dependencies(
            &self,
            visit: &mut impl FnMut(::bevy::asset::UntypedAssetId),
        ) {}
    }
    const _: () = {
        impl ::bevy::reflect::TypePath for WaterMaterial {
            fn type_path() -> &'static str {
                "tempest_forge::world::water::WaterMaterial"
            }
            fn short_type_path() -> &'static str {
                "WaterMaterial"
            }
            fn type_ident() -> Option<&'static str> {
                ::core::option::Option::Some("WaterMaterial")
            }
            fn crate_name() -> Option<&'static str> {
                ::core::option::Option::Some(
                    "tempest_forge::world::water".split(':').next().unwrap(),
                )
            }
            fn module_path() -> Option<&'static str> {
                ::core::option::Option::Some("tempest_forge::world::water")
            }
        }
    };
    struct _WaterMaterialAsBindGroupUniformStructBindGroup0<'a> {
        color: &'a Vec4,
        time: &'a f32,
        camera_position: &'a Vec3,
        resolution: &'a Vec2,
        water_level: &'a f32,
        grid_scale: &'a f32,
        cloudiness: &'a f32,
    }
    const _: fn() = || {};
    const _: fn() = || {};
    const _: fn() = || {};
    const _: fn() = || {};
    const _: fn() = || {};
    const _: fn() = || {};
    const _: fn() = || {};
    impl<'a> bevy::render::render_resource::encase::private::ShaderType
    for _WaterMaterialAsBindGroupUniformStructBindGroup0<'a>
    where
        &'a Vec4: bevy::render::render_resource::encase::private::ShaderType
            + bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderType
            + bevy::render::render_resource::encase::private::ShaderSize,
        &'a Vec3: bevy::render::render_resource::encase::private::ShaderType
            + bevy::render::render_resource::encase::private::ShaderSize,
        &'a Vec2: bevy::render::render_resource::encase::private::ShaderType
            + bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderType
            + bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderType
            + bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderType,
    {
        type ExtraMetadata = bevy::render::render_resource::encase::private::StructMetadata<
            7usize,
        >;
        const METADATA: bevy::render::render_resource::encase::private::Metadata<
            Self::ExtraMetadata,
        > = {
            let struct_alignment = bevy::render::render_resource::encase::private::AlignmentValue::max([
                <&'a Vec4 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
                <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
                <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
                <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
                <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
                <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
                <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment(),
            ]);
            let extra = {
                let mut paddings = [0; 7usize];
                let mut offsets = [0; 7usize];
                let mut offset = 0;
                offset
                    += <&'a Vec4 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                offsets[1usize] = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .round_up(offset);
                let padding = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .padding_needed_for(offset);
                offset += padding;
                paddings[0usize] = padding;
                offset
                    += <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                offsets[2usize] = <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .round_up(offset);
                let padding = <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .padding_needed_for(offset);
                offset += padding;
                paddings[1usize] = padding;
                offset
                    += <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                offsets[3usize] = <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .round_up(offset);
                let padding = <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .padding_needed_for(offset);
                offset += padding;
                paddings[2usize] = padding;
                offset
                    += <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                offsets[4usize] = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .round_up(offset);
                let padding = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .padding_needed_for(offset);
                offset += padding;
                paddings[3usize] = padding;
                offset
                    += <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                offsets[5usize] = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .round_up(offset);
                let padding = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .padding_needed_for(offset);
                offset += padding;
                paddings[4usize] = padding;
                offset
                    += <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                offsets[6usize] = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .round_up(offset);
                let padding = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .alignment()
                    .padding_needed_for(offset);
                offset += padding;
                paddings[5usize] = padding;
                offset
                    += <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                        .get();
                paddings[6usize] = struct_alignment.padding_needed_for(offset);
                bevy::render::render_resource::encase::private::StructMetadata {
                    offsets,
                    paddings,
                }
            };
            let min_size = {
                let mut offset = extra.offsets[7usize - 1];
                offset
                    += <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                        .min_size()
                        .get();
                bevy::render::render_resource::encase::private::SizeValue::new(
                    struct_alignment.round_up(offset),
                )
            };
            bevy::render::render_resource::encase::private::Metadata {
                alignment: struct_alignment,
                has_uniform_min_alignment: true,
                min_size,
                is_pod: false,
                extra,
            }
        };
        const UNIFORM_COMPAT_ASSERT: fn() = || bevy::render::render_resource::encase::private::consume_zsts([
            <&'a Vec4 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a Vec4 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(0usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"color" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            (),
            <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(1usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"time" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            if let ::core::option::Option::Some(min_alignment) = <&'a Vec4 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let prev_offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(1usize - 1);
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(1usize);
                let diff = offset - prev_offset;
                let prev_size = <&'a Vec4 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                    .get();
                let prev_size = min_alignment.round_up(prev_size);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = diff >= prev_size {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset between fields '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"color" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' and '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"time" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be at least " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (currently: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &diff {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(2usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"camera_position" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let prev_offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(2usize - 1);
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(2usize);
                let diff = offset - prev_offset;
                let prev_size = <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                    .get();
                let prev_size = min_alignment.round_up(prev_size);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = diff >= prev_size {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset between fields '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"time" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' and '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"camera_position" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be at least " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (currently: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &diff {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(3usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"resolution" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            if let ::core::option::Option::Some(min_alignment) = <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let prev_offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(3usize - 1);
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(3usize);
                let diff = offset - prev_offset;
                let prev_size = <&'a Vec3 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                    .get();
                let prev_size = min_alignment.round_up(prev_size);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = diff >= prev_size {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset between fields '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"camera_position" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' and '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"resolution" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be at least " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (currently: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &diff {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(4usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"water_level" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            if let ::core::option::Option::Some(min_alignment) = <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let prev_offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(4usize - 1);
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(4usize);
                let diff = offset - prev_offset;
                let prev_size = <&'a Vec2 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                    .get();
                let prev_size = min_alignment.round_up(prev_size);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = diff >= prev_size {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset between fields '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"resolution" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' and '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"water_level" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be at least " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (currently: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &diff {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(5usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"grid_scale" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let prev_offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(5usize - 1);
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(5usize);
                let diff = offset - prev_offset;
                let prev_size = <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                    .get();
                let prev_size = min_alignment.round_up(prev_size);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = diff >= prev_size {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset between fields '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"water_level" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' and '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"grid_scale" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be at least " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (currently: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &diff {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::UNIFORM_COMPAT_ASSERT(),
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(6usize);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = min_alignment.is_aligned(offset) {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset of field '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"cloudiness" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be a multiple of " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (current offset: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &offset {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
            if let ::core::option::Option::Some(min_alignment) = <&'a f32 as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                .uniform_min_alignment()
            {
                let prev_offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(6usize - 1);
                let offset = <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .offset(6usize);
                let diff = offset - prev_offset;
                let prev_size = <&'a f32 as bevy::render::render_resource::encase::private::ShaderSize>::SHADER_SIZE
                    .get();
                let prev_size = min_alignment.round_up(prev_size);
                {
                    #[allow(clippy::equatable_if_let)]
                    if let false = diff >= prev_size {
                        {
                            let mut fmt: ::const_panic::FmtArg = ::const_panic::FmtArg::DEBUG;
                            match &[
                                ::const_panic::StdWrapper(
                                        &match &"offset between fields '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"grid_scale" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' and '" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"cloudiness" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &"' must be at least " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &min_alignment.get() {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &" (currently: " {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &diff {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt),
                                    )
                                    .deref_panic_vals(),
                                ::const_panic::StdWrapper(
                                        &match &")" {
                                            reff => {
                                                ::const_panic::__::PanicFmt::PROOF.infer(reff).coerce(reff)
                                            }
                                        }
                                            .to_panicvals(fmt.set_display().set_alternate(false)),
                                    )
                                    .deref_panic_vals(),
                            ] {
                                args => ::const_panic::concat_panic(args),
                            }
                        }
                    }
                }
            },
        ]);
        fn size(&self) -> ::core::num::NonZeroU64 {
            let mut offset = Self::METADATA.last_offset();
            offset
                += bevy::render::render_resource::encase::private::ShaderType::size(
                        &self.cloudiness,
                    )
                    .get();
            bevy::render::render_resource::encase::private::SizeValue::new(
                    Self::METADATA.alignment().round_up(offset),
                )
                .0
        }
    }
    impl<'a> bevy::render::render_resource::encase::private::WriteInto
    for _WaterMaterialAsBindGroupUniformStructBindGroup0<'a>
    where
        Self: bevy::render::render_resource::encase::private::ShaderType<
            ExtraMetadata = bevy::render::render_resource::encase::private::StructMetadata<
                7usize,
            >,
        >,
        for<'__> &'a Vec4: bevy::render::render_resource::encase::private::WriteInto,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::WriteInto,
        for<'__> &'a Vec3: bevy::render::render_resource::encase::private::WriteInto,
        for<'__> &'a Vec2: bevy::render::render_resource::encase::private::WriteInto,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::WriteInto,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::WriteInto,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::WriteInto,
    {
        #[inline]
        fn write_into<B: bevy::render::render_resource::encase::private::BufferMut>(
            &self,
            writer: &mut bevy::render::render_resource::encase::private::Writer<B>,
        ) {
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.color,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(0usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.time,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(1usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.camera_position,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(2usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.resolution,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(3usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.water_level,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(4usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.grid_scale,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(5usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::WriteInto::write_into(
                &self.cloudiness,
                writer,
            );
            bevy::render::render_resource::encase::private::Writer::advance(
                writer,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(6usize) as ::core::primitive::usize,
            );
        }
    }
    impl<'a> bevy::render::render_resource::encase::private::ReadFrom
    for _WaterMaterialAsBindGroupUniformStructBindGroup0<'a>
    where
        Self: bevy::render::render_resource::encase::private::ShaderType<
            ExtraMetadata = bevy::render::render_resource::encase::private::StructMetadata<
                7usize,
            >,
        >,
        for<'__> &'a Vec4: bevy::render::render_resource::encase::private::ReadFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::ReadFrom,
        for<'__> &'a Vec3: bevy::render::render_resource::encase::private::ReadFrom,
        for<'__> &'a Vec2: bevy::render::render_resource::encase::private::ReadFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::ReadFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::ReadFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::ReadFrom,
    {
        #[inline]
        fn read_from<B: bevy::render::render_resource::encase::private::BufferRef>(
            &mut self,
            reader: &mut bevy::render::render_resource::encase::private::Reader<B>,
        ) {
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.color,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(0usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.time,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(1usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.camera_position,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(2usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.resolution,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(3usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.water_level,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(4usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.grid_scale,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(5usize) as ::core::primitive::usize,
            );
            bevy::render::render_resource::encase::private::ReadFrom::read_from(
                &mut self.cloudiness,
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(6usize) as ::core::primitive::usize,
            );
        }
    }
    impl<'a> bevy::render::render_resource::encase::private::CreateFrom
    for _WaterMaterialAsBindGroupUniformStructBindGroup0<'a>
    where
        Self: bevy::render::render_resource::encase::private::ShaderType<
            ExtraMetadata = bevy::render::render_resource::encase::private::StructMetadata<
                7usize,
            >,
        >,
        for<'__> &'a Vec4: bevy::render::render_resource::encase::private::CreateFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::CreateFrom,
        for<'__> &'a Vec3: bevy::render::render_resource::encase::private::CreateFrom,
        for<'__> &'a Vec2: bevy::render::render_resource::encase::private::CreateFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::CreateFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::CreateFrom,
        for<'__> &'a f32: bevy::render::render_resource::encase::private::CreateFrom,
    {
        #[inline]
        fn create_from<B: bevy::render::render_resource::encase::private::BufferRef>(
            reader: &mut bevy::render::render_resource::encase::private::Reader<B>,
        ) -> Self {
            let color = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(0usize) as ::core::primitive::usize,
            );
            let time = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(1usize) as ::core::primitive::usize,
            );
            let camera_position = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(2usize) as ::core::primitive::usize,
            );
            let resolution = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(3usize) as ::core::primitive::usize,
            );
            let water_level = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(4usize) as ::core::primitive::usize,
            );
            let grid_scale = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(5usize) as ::core::primitive::usize,
            );
            let cloudiness = bevy::render::render_resource::encase::private::CreateFrom::create_from(
                reader,
            );
            bevy::render::render_resource::encase::private::Reader::advance(
                reader,
                <Self as bevy::render::render_resource::encase::private::ShaderType>::METADATA
                    .padding(6usize) as ::core::primitive::usize,
            );
            {
                let mut uninit_struct = ::core::mem::MaybeUninit::<Self>::uninit();
                let ptr = ::core::mem::MaybeUninit::as_mut_ptr(&mut uninit_struct);
                let field_ptr = unsafe { &raw mut (*ptr).color };
                unsafe { field_ptr.write(color) };
                let field_ptr = unsafe { &raw mut (*ptr).time };
                unsafe { field_ptr.write(time) };
                let field_ptr = unsafe { &raw mut (*ptr).camera_position };
                unsafe { field_ptr.write(camera_position) };
                let field_ptr = unsafe { &raw mut (*ptr).resolution };
                unsafe { field_ptr.write(resolution) };
                let field_ptr = unsafe { &raw mut (*ptr).water_level };
                unsafe { field_ptr.write(water_level) };
                let field_ptr = unsafe { &raw mut (*ptr).grid_scale };
                unsafe { field_ptr.write(grid_scale) };
                let field_ptr = unsafe { &raw mut (*ptr).cloudiness };
                unsafe { field_ptr.write(cloudiness) };
                unsafe { ::core::mem::MaybeUninit::assume_init(uninit_struct) }
            }
        }
    }
    impl<'a> bevy::render::render_resource::encase::private::ShaderSize
    for _WaterMaterialAsBindGroupUniformStructBindGroup0<'a>
    where
        &'a Vec4: bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderSize,
        &'a Vec3: bevy::render::render_resource::encase::private::ShaderSize,
        &'a Vec2: bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderSize,
        &'a f32: bevy::render::render_resource::encase::private::ShaderSize,
    {}
    impl ::bevy::render::render_resource::AsBindGroup for WaterMaterial {
        type Data = ();
        type Param = (
            ::bevy::ecs::system::lifetimeless::SRes<
                ::bevy::render::render_asset::RenderAssets<
                    ::bevy::render::texture::GpuImage,
                >,
            >,
            ::bevy::ecs::system::lifetimeless::SRes<
                ::bevy::render::texture::FallbackImage,
            >,
            ::bevy::ecs::system::lifetimeless::SRes<
                ::bevy::render::render_asset::RenderAssets<
                    ::bevy::render::storage::GpuShaderStorageBuffer,
                >,
            >,
        );
        fn label() -> &'static str {
            "WaterMaterial"
        }
        fn unprepared_bind_group(
            &self,
            layout: &::bevy::render::render_resource::BindGroupLayout,
            render_device: &::bevy::render::renderer::RenderDevice,
            (
                images,
                fallback_image,
                storage_buffers,
            ): &mut ::bevy::ecs::system::SystemParamItem<'_, '_, Self::Param>,
            force_no_bindless: bool,
        ) -> Result<
            ::bevy::render::render_resource::UnpreparedBindGroup,
            ::bevy::render::render_resource::AsBindGroupError,
        > {
            let (uniform_binding_type, uniform_buffer_usages) = (
                ::bevy::render::render_resource::BufferBindingType::Uniform,
                ::bevy::render::render_resource::BufferUsages::UNIFORM,
            );
            let bindings = ::bevy::render::render_resource::BindingResources(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            (
                                2u32,
                                ::bevy::render::render_resource::OwnedBindingResource::Sampler(
                                    ::bevy::render::render_resource::SamplerBindingType::Filtering,
                                    {
                                        let handle: Option<
                                            &::bevy::asset::Handle<::bevy::image::Image>,
                                        > = (&self.reflection_texture).into();
                                        if let Some(handle) = handle {
                                            let image = images
                                                .get(handle)
                                                .ok_or_else(|| {
                                                    ::bevy::render::render_resource::AsBindGroupError::RetryNextUpdate
                                                })?;
                                            let Some(sample_type) = image
                                                .texture_format
                                                .sample_type(None, Some(render_device.features())) else {
                                                return Err(
                                                    ::bevy::render::render_resource::AsBindGroupError::InvalidSamplerType(
                                                        2u32,
                                                        "None".to_string(),
                                                        ::alloc::__export::must_use({
                                                            ::alloc::fmt::format(
                                                                format_args!(
                                                                    "{0:?}",
                                                                    [
                                                                        ::bevy::render::render_resource::TextureSampleType::Float {
                                                                            filterable: true,
                                                                        },
                                                                    ],
                                                                ),
                                                            )
                                                        }),
                                                    ),
                                                );
                                            };
                                            let valid = [
                                                ::bevy::render::render_resource::TextureSampleType::Float {
                                                    filterable: true,
                                                },
                                            ]
                                                .contains(&sample_type);
                                            if !valid {
                                                return Err(
                                                    ::bevy::render::render_resource::AsBindGroupError::InvalidSamplerType(
                                                        2u32,
                                                        ::alloc::__export::must_use({
                                                            ::alloc::fmt::format(format_args!("{0:?}", sample_type))
                                                        }),
                                                        ::alloc::__export::must_use({
                                                            ::alloc::fmt::format(
                                                                format_args!(
                                                                    "{0:?}",
                                                                    [
                                                                        ::bevy::render::render_resource::TextureSampleType::Float {
                                                                            filterable: true,
                                                                        },
                                                                    ],
                                                                ),
                                                            )
                                                        }),
                                                    ),
                                                );
                                            }
                                            image.sampler.clone()
                                        } else {
                                            match ::bevy::render::render_resource::TextureViewDimension::D2 {
                                                ::bevy::render::render_resource::TextureViewDimension::D1 => {
                                                    &fallback_image.d1
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::D2 => {
                                                    &fallback_image.d2
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::D2Array => {
                                                    &fallback_image.d2_array
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::Cube => {
                                                    &fallback_image.cube
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::CubeArray => {
                                                    &fallback_image.cube_array
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::D3 => {
                                                    &fallback_image.d3
                                                }
                                            }
                                                .sampler
                                                .clone()
                                        }
                                    },
                                ),
                            ),
                            (
                                1u32,
                                ::bevy::render::render_resource::OwnedBindingResource::TextureView(
                                    ::bevy::render::render_resource::TextureViewDimension::D2,
                                    {
                                        let handle: Option<
                                            &::bevy::asset::Handle<::bevy::image::Image>,
                                        > = (&self.reflection_texture).into();
                                        if let Some(handle) = handle {
                                            images
                                                .get(handle)
                                                .ok_or_else(|| {
                                                    ::bevy::render::render_resource::AsBindGroupError::RetryNextUpdate
                                                })?
                                                .texture_view
                                                .clone()
                                        } else {
                                            match ::bevy::render::render_resource::TextureViewDimension::D2 {
                                                ::bevy::render::render_resource::TextureViewDimension::D1 => {
                                                    &fallback_image.d1
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::D2 => {
                                                    &fallback_image.d2
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::D2Array => {
                                                    &fallback_image.d2_array
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::Cube => {
                                                    &fallback_image.cube
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::CubeArray => {
                                                    &fallback_image.cube_array
                                                }
                                                ::bevy::render::render_resource::TextureViewDimension::D3 => {
                                                    &fallback_image.d3
                                                }
                                            }
                                                .texture_view
                                                .clone()
                                        }
                                    },
                                ),
                            ),
                            (
                                3u32,
                                ::bevy::render::render_resource::OwnedBindingResource::Buffer({
                                    let handle: &::bevy::asset::Handle<
                                        ::bevy::render::storage::ShaderStorageBuffer,
                                    > = (&self.height_buffer);
                                    storage_buffers
                                        .get(handle)
                                        .ok_or_else(|| {
                                            ::bevy::render::render_resource::AsBindGroupError::RetryNextUpdate
                                        })?
                                        .buffer
                                        .clone()
                                }),
                            ),
                            {
                                let mut buffer = ::bevy::render::render_resource::encase::UniformBuffer::new(
                                    Vec::new(),
                                );
                                buffer
                                    .write(
                                        &_WaterMaterialAsBindGroupUniformStructBindGroup0 {
                                            color: &self.color,
                                            time: &self.time,
                                            camera_position: &self.camera_position,
                                            resolution: &self.resolution,
                                            water_level: &self.water_level,
                                            grid_scale: &self.grid_scale,
                                            cloudiness: &self.cloudiness,
                                        },
                                    )
                                    .unwrap();
                                (
                                    0u32,
                                    ::bevy::render::render_resource::OwnedBindingResource::Buffer(
                                        render_device
                                            .create_buffer_with_data(
                                                &::bevy::render::render_resource::BufferInitDescriptor {
                                                    label: None,
                                                    usage: uniform_buffer_usages,
                                                    contents: buffer.as_ref(),
                                                },
                                            ),
                                    ),
                                )
                            },
                        ],
                    ),
                ),
            );
            Ok(::bevy::render::render_resource::UnpreparedBindGroup {
                bindings,
            })
        }
        #[allow(clippy::unused_unit)]
        fn bind_group_data(&self) -> Self::Data {
            ()
        }
        fn bind_group_layout_entries(
            render_device: &::bevy::render::renderer::RenderDevice,
            force_no_bindless: bool,
        ) -> Vec<::bevy::render::render_resource::BindGroupLayoutEntry> {
            let actual_bindless_slot_count: Option<::core::num::NonZeroU32> = None;
            let (uniform_binding_type, uniform_buffer_usages) = (
                ::bevy::render::render_resource::BufferBindingType::Uniform,
                ::bevy::render::render_resource::BufferUsages::UNIFORM,
            );
            let mut bind_group_layout_entries = Vec::new();
            match actual_bindless_slot_count {
                Some(bindless_slot_count) => {
                    let bindless_index_table_range = ::bevy::render::render_resource::BindlessIndex(
                        0,
                    )..::bevy::render::render_resource::BindlessIndex(3u32);
                    bind_group_layout_entries
                        .extend(
                            ::bevy::render::render_resource::create_bindless_bind_group_layout_entries(
                                    bindless_index_table_range.end.0
                                        - bindless_index_table_range.start.0,
                                    bindless_slot_count.into(),
                                    ::bevy::render::render_resource::BindingNumber(0),
                                )
                                .into_iter(),
                        );
                }
                None => {
                    bind_group_layout_entries
                        .push(::bevy::render::render_resource::BindGroupLayoutEntry {
                            binding: 1u32,
                            visibility: ::bevy::render::render_resource::ShaderStages::VERTEX
                                | ::bevy::render::render_resource::ShaderStages::FRAGMENT,
                            ty: ::bevy::render::render_resource::BindingType::Texture {
                                multisampled: false,
                                sample_type: ::bevy::render::render_resource::TextureSampleType::Float {
                                    filterable: true,
                                },
                                view_dimension: ::bevy::render::render_resource::TextureViewDimension::D2,
                            },
                            count: actual_bindless_slot_count,
                        });
                    bind_group_layout_entries
                        .push(::bevy::render::render_resource::BindGroupLayoutEntry {
                            binding: 2u32,
                            visibility: ::bevy::render::render_resource::ShaderStages::VERTEX
                                | ::bevy::render::render_resource::ShaderStages::FRAGMENT,
                            ty: ::bevy::render::render_resource::BindingType::Sampler(
                                ::bevy::render::render_resource::SamplerBindingType::Filtering,
                            ),
                            count: actual_bindless_slot_count,
                        });
                    bind_group_layout_entries
                        .push(::bevy::render::render_resource::BindGroupLayoutEntry {
                            binding: 3u32,
                            visibility: ::bevy::render::render_resource::ShaderStages::VERTEX
                                | ::bevy::render::render_resource::ShaderStages::FRAGMENT,
                            ty: ::bevy::render::render_resource::BindingType::Buffer {
                                ty: ::bevy::render::render_resource::BufferBindingType::Storage {
                                    read_only: true,
                                },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: actual_bindless_slot_count,
                        });
                    bind_group_layout_entries
                        .push(::bevy::render::render_resource::BindGroupLayoutEntry {
                            binding: 0u32,
                            visibility: ::bevy::render::render_resource::ShaderStages::FRAGMENT
                                | ::bevy::render::render_resource::ShaderStages::VERTEX
                                | ::bevy::render::render_resource::ShaderStages::COMPUTE,
                            ty: ::bevy::render::render_resource::BindingType::Buffer {
                                ty: uniform_binding_type,
                                has_dynamic_offset: false,
                                min_binding_size: Some(
                                    <_WaterMaterialAsBindGroupUniformStructBindGroup0 as ::bevy::render::render_resource::ShaderType>::min_size(),
                                ),
                            },
                            count: actual_bindless_slot_count,
                        });
                }
            };
            bind_group_layout_entries
        }
        fn bindless_descriptor() -> Option<
            ::bevy::render::render_resource::BindlessDescriptor,
        > {
            None
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for WaterMaterial {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "color",
                "time",
                "camera_position",
                "resolution",
                "water_level",
                "grid_scale",
                "cloudiness",
                "reflection_texture",
                "height_buffer",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.color,
                &self.time,
                &self.camera_position,
                &self.resolution,
                &self.water_level,
                &self.grid_scale,
                &self.cloudiness,
                &self.reflection_texture,
                &&self.height_buffer,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "WaterMaterial",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for WaterMaterial {
        #[inline]
        fn clone(&self) -> WaterMaterial {
            WaterMaterial {
                color: ::core::clone::Clone::clone(&self.color),
                time: ::core::clone::Clone::clone(&self.time),
                camera_position: ::core::clone::Clone::clone(&self.camera_position),
                resolution: ::core::clone::Clone::clone(&self.resolution),
                water_level: ::core::clone::Clone::clone(&self.water_level),
                grid_scale: ::core::clone::Clone::clone(&self.grid_scale),
                cloudiness: ::core::clone::Clone::clone(&self.cloudiness),
                reflection_texture: ::core::clone::Clone::clone(
                    &self.reflection_texture,
                ),
                height_buffer: ::core::clone::Clone::clone(&self.height_buffer),
            }
        }
    }
    impl WaterMaterial {
        pub fn new(color: Color) -> Self {
            let c = color.to_linear();
            Self {
                color: Vec4::new(c.red, c.green, c.blue, c.alpha),
                time: 0.0,
                camera_position: Vec3::ZERO,
                resolution: Vec2::new(1920.0, 1080.0),
                water_level: 15.0,
                grid_scale: 512.0 / WATER_GRID_SIZE as f32,
                cloudiness: 0.0,
                reflection_texture: None,
                height_buffer: Handle::default(),
            }
        }
    }
    impl Material for WaterMaterial {
        fn vertex_shader() -> ShaderRef {
            "shaders/water_material.wgsl".into()
        }
        fn fragment_shader() -> ShaderRef {
            "shaders/water_material.wgsl".into()
        }
        fn alpha_mode(&self) -> AlphaMode {
            AlphaMode::Blend
        }
    }
    pub fn create_water_mesh(size: f32, grid_size: usize) -> Mesh {
        let vertices_per_side = grid_size + 1;
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        let step = size / grid_size as f32;
        let half_size = size * 0.5;
        for y in 0..vertices_per_side {
            for x in 0..vertices_per_side {
                let x_pos = (x as f32 * step) - half_size;
                let z_pos = (y as f32 * step) - half_size;
                positions.push([x_pos, 0.0, z_pos]);
                normals.push([0.0, 1.0, 0.0]);
                uvs.push([x as f32 / grid_size as f32, y as f32 / grid_size as f32]);
            }
        }
        for y in 0..grid_size {
            for x in 0..grid_size {
                let base = (y * vertices_per_side + x) as u32;
                indices.push(base);
                indices.push(base + vertices_per_side as u32);
                indices.push(base + 1);
                indices.push(base + 1);
                indices.push(base + vertices_per_side as u32);
                indices.push(base + vertices_per_side as u32 + 1);
            }
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
    fn setup_water(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut water_materials: ResMut<Assets<WaterMaterial>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let size = 256.0;
        let grid_len = WATER_GRID_SIZE as usize;
        let _size_f = size;
        let extent = Extent3d {
            width: 960,
            height: 540,
            depth_or_array_layers: 1,
        };
        let mut reflection_image = Image {
            texture_descriptor: TextureDescriptor {
                label: Some("water_reflection"),
                size: extent,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        reflection_image.resize(extent);
        let reflection_handle = images.add(reflection_image);
        commands
            .insert_resource(ReflectionTarget {
                image: reflection_handle.clone(),
            });
        commands
            .spawn((
                Name::new("WaterReflectionCamera"),
                ReflectionCamera,
                Camera3d::default(),
                Camera {
                    order: -1,
                    invert_culling: true,
                    clear_color: ClearColorConfig::Default,
                    ..default()
                },
                RenderTarget::Image(reflection_handle.clone().into()),
                Projection::Perspective(PerspectiveProjection {
                    fov: 90.0f32.to_radians(),
                    far: 2000.0,
                    near: 0.1,
                    ..default()
                }),
                Transform::default(),
                RenderLayers::layer(0),
            ));
        let mesh_handle = meshes.add(create_water_mesh(size, grid_len));
        let material_handle = water_materials
            .add(WaterMaterial {
                color: {
                    let c = Color::srgba(0.02, 0.32, 0.78, 0.26).to_linear();
                    Vec4::new(c.red, c.green, c.blue, c.alpha)
                },
                time: 0.0,
                camera_position: Vec3::ZERO,
                resolution: Vec2::new(1920.0, 1080.0),
                water_level: 15.0,
                grid_scale: 512.0 / WATER_GRID_SIZE as f32,
                cloudiness: 0.0,
                reflection_texture: Some(reflection_handle.clone()),
                height_buffer: Handle::default(),
            });
        commands
            .spawn((
                Name::new("SimulatedWaterPlane"),
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_handle),
                Transform::from_xyz(0.0, 15.1, 0.0),
                WaterSimData::new(grid_len, size),
                WaterMesh { handle: mesh_handle },
                RenderLayers::layer(1),
            ));
    }
    fn cleanup_water(
        mut commands: Commands,
        q: Query<
            Entity,
            Or<(With<ReflectionCamera>, With<WaterSimData>, With<WaterMesh>)>,
        >,
    ) {
        for e in &q {
            commands.entity(e).despawn();
        }
    }
    #[allow(dead_code)]
    fn water_sim(time: Res<Time>, mut query: Query<&mut WaterSimData>) {
        let delta_time = time.delta_secs().min(0.03);
        let gravity: f32 = 12.0;
        let friction: f32 = 0.965;
        for mut water_data in query.iter_mut() {
            let grid_len = water_data.grid_len;
            for i in 0..grid_len {
                water_data.set_flow_x(0, i, 0.0);
                water_data.set_flow_x(grid_len - 1, i, 0.0);
                water_data.set_flow_y(i, 0, 0.0);
                water_data.set_flow_y(i, grid_len - 1, 0.0);
            }
            for x in 0..grid_len {
                for y in 0..grid_len {
                    if x > 0 {
                        let source_has_wall = water_data.is_wall(x - 1, y);
                        let dest_has_wall = water_data.is_wall(x, y);
                        let height_diff = water_data.get_height(x - 1, y)
                            - water_data.get_height(x, y);
                        if !source_has_wall && !dest_has_wall {
                            let current_flow = water_data.get_flow_x(x, y);
                            let new_flow = current_flow * friction.powf(delta_time)
                                + height_diff * gravity * delta_time;
                            water_data.set_flow_x(x, y, new_flow);
                        } else {
                            water_data.set_flow_x(x, y, 0.0);
                        }
                    } else {
                        water_data.set_flow_x(x, y, 0.0);
                    }
                    if y > 0 {
                        let source_has_wall = water_data.is_wall(x, y - 1);
                        let dest_has_wall = water_data.is_wall(x, y);
                        let height_diff = water_data.get_height(x, y - 1)
                            - water_data.get_height(x, y);
                        if !source_has_wall && !dest_has_wall {
                            let current_flow = water_data.get_flow_y(x, y);
                            let new_flow = current_flow * friction.powf(delta_time)
                                + height_diff * gravity * delta_time;
                            water_data.set_flow_y(x, y, new_flow);
                        } else {
                            water_data.set_flow_y(x, y, 0.0);
                        }
                    } else {
                        water_data.set_flow_y(x, y, 0.0);
                    }
                }
            }
            for x in 0..grid_len {
                for y in 0..grid_len {
                    if water_data.is_wall(x, y) {
                        continue;
                    }
                    let mut total_outflow = 0.0;
                    total_outflow += 0.0f32.max(-water_data.get_flow_x(x, y));
                    total_outflow += 0.0f32.max(-water_data.get_flow_y(x, y));
                    if x < grid_len - 1 {
                        total_outflow += 0.0f32.max(water_data.get_flow_x(x + 1, y));
                    }
                    if y < grid_len - 1 {
                        total_outflow += 0.0f32.max(water_data.get_flow_y(x, y + 1));
                    }
                    let max_outflow = water_data.get_height(x, y) / delta_time;
                    if total_outflow > 0.0 {
                        let scale = 1.0f32.min(max_outflow / total_outflow);
                        if water_data.get_flow_x(x, y) < 0.0 {
                            let val = water_data.get_flow_x(x, y) * scale;
                            water_data.set_flow_x(x, y, val);
                        }
                        if water_data.get_flow_y(x, y) < 0.0 {
                            let val = water_data.get_flow_y(x, y) * scale;
                            water_data.set_flow_y(x, y, val);
                        }
                        if x < grid_len - 1 && water_data.get_flow_x(x + 1, y) > 0.0 {
                            let val = water_data.get_flow_x(x + 1, y) * scale;
                            water_data.set_flow_x(x + 1, y, val);
                        }
                        if y < grid_len - 1 && water_data.get_flow_y(x, y + 1) > 0.0 {
                            let val = water_data.get_flow_y(x, y + 1) * scale;
                            water_data.set_flow_y(x, y + 1, val);
                        }
                    }
                }
            }
            for x in 0..grid_len {
                for y in 0..grid_len {
                    let mut height_change = 0.0;
                    let can_receive_from_left = x > 0 && !water_data.is_wall(x - 1, y)
                        && !water_data.is_wall(x, y);
                    if can_receive_from_left {
                        height_change += water_data.get_flow_x(x, y);
                    }
                    let can_receive_from_top = y > 0 && !water_data.is_wall(x, y - 1)
                        && !water_data.is_wall(x, y);
                    if can_receive_from_top {
                        height_change += water_data.get_flow_y(x, y);
                    }
                    let can_flow_right = x < grid_len - 1
                        && !water_data.is_wall(x + 1, y);
                    if can_flow_right {
                        height_change -= water_data.get_flow_x(x + 1, y);
                    }
                    let can_flow_bottom = y < grid_len - 1
                        && !water_data.is_wall(x, y + 1);
                    if can_flow_bottom {
                        height_change -= water_data.get_flow_y(x, y + 1);
                    }
                    let current_height = water_data.get_height(x, y);
                    let mut new_height = current_height + height_change * delta_time;
                    new_height = new_height.max(0.1);
                    if water_data.is_wall(x, y) {
                        new_height = 0.1;
                    }
                    water_data.set_height(x, y, new_height);
                }
            }
        }
    }
    #[allow(dead_code)]
    fn animate_water_mesh(
        mut meshes: ResMut<Assets<Mesh>>,
        mut query: Query<(&WaterMesh, &mut WaterSimData)>,
    ) {
        for (water_mesh, mut water_data) in query.iter_mut() {
            if !water_data.dirty {
                continue;
            }
            if let Some(mesh) = meshes.get_mut(&water_mesh.handle) {
                let grid_len = water_data.grid_len;
                let vertices_per_side = grid_len + 1;
                let grid_scale = water_data.size / grid_len as f32;
                if let Some(vertex_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                    && let VertexAttributeValues::Float32x3(positions) = vertex_attr
                {
                    for (i, pos) in positions.iter_mut().enumerate() {
                        let x_idx = i % vertices_per_side;
                        let y_idx = i / vertices_per_side;
                        let grid_x = x_idx.min(grid_len - 1);
                        let grid_y = y_idx.min(grid_len - 1);
                        pos[1] = water_data.get_height(grid_x, grid_y) - 1.0;
                    }
                }
                let positions_copy = if let Some(
                    VertexAttributeValues::Float32x3(positions),
                ) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    Some(positions.clone())
                } else {
                    None
                };
                if let Some(positions) = positions_copy
                    && let Some(norm_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
                    && let VertexAttributeValues::Float32x3(normals) = norm_attr
                {
                    for i in 0..normals.len() {
                        let x = i % vertices_per_side;
                        let y = i / vertices_per_side;
                        let mut dx = 0.0;
                        let mut dy = 0.0;
                        if x > 0 && x < vertices_per_side - 1 {
                            let h_left = positions[i - 1][1];
                            let h_right = positions[i + 1][1];
                            dx = (h_right - h_left) / (2.0 * grid_scale);
                        }
                        if y > 0 && y < vertices_per_side - 1 {
                            let h_up = positions[i - vertices_per_side][1];
                            let h_down = positions[i + vertices_per_side][1];
                            dy = (h_down - h_up) / (2.0 * grid_scale);
                        }
                        let normal = Vec3::new(-dx, 1.0, -dy).normalize();
                        normals[i] = [normal.x, normal.y, normal.z];
                    }
                }
            }
            water_data.dirty = false;
        }
    }
    fn update_water_material(
        time: Res<Time>,
        player_q: Query<&Transform, With<Player>>,
        pivot_q: Query<&Transform, (With<CameraPivot>, Without<Player>)>,
        camera_q: Query<
            &Transform,
            (With<MainCamera>, Without<Player>, Without<CameraPivot>),
        >,
        water_query: Query<
            (&Transform, &MeshMaterial3d<WaterMaterial>),
            With<WaterSimData>,
        >,
        windows: Query<&Window>,
        mut water_materials: ResMut<Assets<WaterMaterial>>,
        weather: Option<Res<super::weather::WeatherManager>>,
        gpu_handles: Option<Res<WaterGpuHandles>>,
    ) {
        let Ok(player_transform) = player_q.single() else {
            return;
        };
        let Ok(pivot_transform) = pivot_q.single() else {
            return;
        };
        let Ok(camera_transform) = camera_q.single() else {
            return;
        };
        let camera_position = player_transform.translation
            + player_transform.rotation
                * (pivot_transform.translation
                    + player_transform.rotation * camera_transform.translation);
        let resolution = windows
            .single()
            .map(|w| Vec2::new(w.physical_width() as f32, w.physical_height() as f32))
            .unwrap_or(Vec2::new(1920.0, 1080.0));
        let cloudiness = weather.map(|w| w.cloudiness).unwrap_or(0.0);
        if let Some(handles) = &gpu_handles {
            let height_buf_handle = handles.height_current.clone();
            for (water_transform, mat_handle) in water_query.iter() {
                if let Some(material) = water_materials.get_mut(&mat_handle.0) {
                    material.time = time.elapsed_secs();
                    material.camera_position = camera_position;
                    material.resolution = resolution;
                    material.cloudiness = cloudiness;
                    material.water_level = water_transform.translation.y;
                    material.height_buffer = height_buf_handle.clone();
                }
            }
        } else {
            for (water_transform, mat_handle) in water_query.iter() {
                if let Some(material) = water_materials.get_mut(&mat_handle.0) {
                    material.time = time.elapsed_secs();
                    material.camera_position = camera_position;
                    material.resolution = resolution;
                    material.cloudiness = cloudiness;
                    material.water_level = water_transform.translation.y;
                }
            }
        }
    }
    fn sync_reflection_camera(
        player_q: Query<&Transform, With<Player>>,
        pivot_q: Query<&Transform, (With<CameraPivot>, Without<Player>)>,
        main_camera_q: Query<
            (&Transform, &Projection),
            (With<MainCamera>, Without<Player>, Without<CameraPivot>),
        >,
        mut refl_camera_q: Query<
            (&mut Transform, &mut Projection),
            (
                With<ReflectionCamera>,
                Without<Player>,
                Without<CameraPivot>,
                Without<MainCamera>,
            ),
        >,
        water_query: Query<
            &Transform,
            (
                With<WaterSimData>,
                Without<Player>,
                Without<CameraPivot>,
                Without<MainCamera>,
                Without<ReflectionCamera>,
            ),
        >,
        windows: Query<&Window>,
        reflection_target: Res<ReflectionTarget>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let Ok(player_transform) = player_q.single() else {
            return;
        };
        let Ok(pivot_transform) = pivot_q.single() else {
            return;
        };
        let Ok((main_transform, main_proj)) = main_camera_q.single() else {
            return;
        };
        let Ok((mut refl_transform, mut refl_proj)) = refl_camera_q.single_mut() else {
            return;
        };
        let Ok(window) = windows.single() else { return };
        let width = (window.physical_width() / 2).max(1);
        let height = (window.physical_height() / 2).max(1);
        if let Some(image) = images.get_mut(&reflection_target.image)
            && (image.texture_descriptor.size.width != width
                || image.texture_descriptor.size.height != height)
        {
            let new_extent = Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            image.resize(new_extent);
        }
        let water_y = water_query.iter().next().map(|t| t.translation.y).unwrap_or(15.1);
        let main_global_rotation = player_transform.rotation * pivot_transform.rotation
            * main_transform.rotation;
        let main_global_translation = player_transform.translation
            + player_transform.rotation
                * (pivot_transform.translation
                    + player_transform.rotation * main_transform.translation);
        let refl_y = 2.0 * water_y - main_global_translation.y;
        refl_transform.translation = Vec3::new(
            main_global_translation.x,
            refl_y,
            main_global_translation.z,
        );
        refl_transform.scale = Vec3::ONE;
        refl_transform.rotation = Quat::from_xyzw(
            -main_global_rotation.x,
            main_global_rotation.y,
            -main_global_rotation.z,
            main_global_rotation.w,
        );
        *refl_proj = main_proj.clone();
        if let Projection::Perspective(ref mut persp) = *refl_proj {
            let main_y = main_global_translation.y;
            if main_y > water_y {
                let d = water_y - refl_y;
                persp.near = (d - 0.15).max(0.1);
            } else {
                persp.near = 0.1;
            }
            persp.near_clip_plane = Vec4::new(0.0, 0.0, -1.0, -persp.near);
        }
    }
    fn pack_wall_mask(wall_mask: &[bool]) -> Vec<u32> {
        let cell_count = wall_mask.len();
        let size_u32 = (cell_count * 4) / 32;
        let size_u32 = if size_u32 == 0 { 4 } else { size_u32 };
        let mut packed = ::alloc::vec::from_elem(0u32, size_u32);
        for (idx, &is_wall) in wall_mask.iter().enumerate() {
            if is_wall {
                let packed_idx = idx / 32;
                let bit_idx = idx % 32;
                packed[packed_idx] |= 1 << bit_idx;
            }
        }
        packed
    }
    fn update_water_wall_mask(
        voxel_world: VoxelWorld<NoiseGenerator>,
        mut query: Query<(&mut WaterSimData, &Transform)>,
        time: Res<Time>,
        mut timer: Local<f32>,
        gpu_handles: Option<Res<WaterGpuHandles>>,
        mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    ) {
        *timer += time.delta_secs();
        if *timer < 0.25 {
            return;
        }
        *timer = 0.0;
        for (mut water_data, transform) in query.iter_mut() {
            let grid_len = water_data.grid_len;
            let half_size = water_data.size * 0.5;
            let cell_size = water_data.size / grid_len as f32;
            let center = transform.translation;
            for x in 0..grid_len {
                for y in 0..grid_len {
                    if x == 0 || x == grid_len - 1 || y == 0 || y == grid_len - 1 {
                        water_data.set_wall(x, y, true);
                        continue;
                    }
                    let world_x = center.x - half_size + (x as f32 * cell_size);
                    let world_z = center.z - half_size + (y as f32 * cell_size);
                    let voxel_pos = IVec3::new(
                        world_x.round() as i32,
                        15,
                        world_z.round() as i32,
                    );
                    let voxel = voxel_world.get_voxel(voxel_pos);
                    let is_solid_terrain = match voxel {
                        WorldVoxel::Solid(mat_id) => mat_id != 1,
                        WorldVoxel::Air => false,
                        WorldVoxel::Unset => false,
                    };
                    water_data.set_wall(x, y, is_solid_terrain);
                }
            }
            let packed = pack_wall_mask(&water_data.wall_mask);
            if let Some(handles) = &gpu_handles
                && let Some(buf) = buffers.get_mut(&handles.wall_mask)
            {
                buf.data = Some(bytemuck::cast_slice::<u32, u8>(&packed).to_vec());
            }
        }
    }
    fn center_water_on_player(
        player_query: Query<&Transform, (With<PhysicsState>, Without<WaterMesh>)>,
        mut water_query: Query<(&mut Transform, &mut WaterSimData), With<WaterMesh>>,
        gpu_handles: Option<Res<WaterGpuHandles>>,
        mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    ) {
        let Ok(player_transform) = player_query.single() else {
            return;
        };
        let Ok((mut water_transform, mut water_data)) = water_query.single_mut() else {
            return;
        };
        let grid_len = water_data.grid_len;
        let size = water_data.size;
        let cell_size = size / grid_len as f32;
        let target_x = (player_transform.translation.x / cell_size).round() * cell_size;
        let target_z = (player_transform.translation.z / cell_size).round() * cell_size;
        let dx = target_x - water_transform.translation.x;
        let dz = target_z - water_transform.translation.z;
        let shift_x = (dx / cell_size).round() as i32;
        let shift_z = (dz / cell_size).round() as i32;
        if shift_x != 0 || shift_z != 0 {
            let mut new_height = ::alloc::vec::from_elem(1.0, grid_len * grid_len);
            let mut new_flow_x = ::alloc::vec::from_elem(0.0, grid_len * grid_len);
            let mut new_flow_y = ::alloc::vec::from_elem(0.0, grid_len * grid_len);
            let mut new_wall_mask = ::alloc::vec::from_elem(false, grid_len * grid_len);
            for x in 0..grid_len {
                for y in 0..grid_len {
                    let old_x = x as i32 + shift_x;
                    let old_y = y as i32 + shift_z;
                    let new_idx = x * grid_len + y;
                    if old_x >= 0 && old_x < grid_len as i32 && old_y >= 0
                        && old_y < grid_len as i32
                    {
                        let old_idx = (old_x as usize) * grid_len + (old_y as usize);
                        new_height[new_idx] = water_data.height[old_idx];
                        new_flow_x[new_idx] = water_data.flow_x[old_idx];
                        new_flow_y[new_idx] = water_data.flow_y[old_idx];
                        new_wall_mask[new_idx] = water_data.wall_mask[old_idx];
                    }
                }
            }
            water_data.height = new_height;
            water_data.flow_x = new_flow_x;
            water_data.flow_y = new_flow_y;
            water_data.wall_mask = new_wall_mask;
            water_data.dirty = true;
            water_transform.translation.x = target_x;
            water_transform.translation.z = target_z;
            if target_x >= 5000.0 {
                water_transform.translation.y = -1000.0;
            } else {
                water_transform.translation.y = 15.1;
            }
            if let Some(handles) = &gpu_handles {
                if let Some(buf) = buffers.get_mut(&handles.height_current) {
                    buf.data = Some(
                        bytemuck::cast_slice::<f32, u8>(&water_data.height).to_vec(),
                    );
                }
                if let Some(buf) = buffers.get_mut(&handles.height_next) {
                    buf.data = Some(
                        bytemuck::cast_slice::<f32, u8>(&water_data.height).to_vec(),
                    );
                }
                if let Some(buf) = buffers.get_mut(&handles.flow_x_current) {
                    buf.data = Some(
                        bytemuck::cast_slice::<f32, u8>(&water_data.flow_x).to_vec(),
                    );
                }
                if let Some(buf) = buffers.get_mut(&handles.flow_x_next) {
                    buf.data = Some(
                        bytemuck::cast_slice::<f32, u8>(&water_data.flow_x).to_vec(),
                    );
                }
                if let Some(buf) = buffers.get_mut(&handles.flow_y_current) {
                    buf.data = Some(
                        bytemuck::cast_slice::<f32, u8>(&water_data.flow_y).to_vec(),
                    );
                }
                if let Some(buf) = buffers.get_mut(&handles.flow_y_next) {
                    buf.data = Some(
                        bytemuck::cast_slice::<f32, u8>(&water_data.flow_y).to_vec(),
                    );
                }
                let packed = pack_wall_mask(&water_data.wall_mask);
                if let Some(buf) = buffers.get_mut(&handles.wall_mask) {
                    buf.data = Some(bytemuck::cast_slice::<u32, u8>(&packed).to_vec());
                }
            }
        }
    }
    pub struct WaterInteractor {
        pub last_position: Vec3,
        pub is_player: bool,
        pub mass: f32,
    }
    impl ::bevy::ecs::component::Component for WaterInteractor
    where
        Self: Send + Sync + 'static,
    {
        const STORAGE_TYPE: ::bevy::ecs::component::StorageType = ::bevy::ecs::component::StorageType::Table;
        type Mutability = ::bevy::ecs::component::Mutable;
        fn register_required_components(
            _requiree: ::bevy::ecs::component::ComponentId,
            required_components: &mut ::bevy::ecs::component::RequiredComponentsRegistrator,
        ) {}
        fn clone_behavior() -> ::bevy::ecs::component::ComponentCloneBehavior {
            use ::bevy::ecs::component::{
                DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone,
            };
            (&&&::bevy::ecs::component::DefaultCloneBehaviorSpecialization::<
                Self,
            >::default())
                .default_clone_behavior()
        }
        fn relationship_accessor() -> Option<
            ::bevy::ecs::relationship::ComponentRelationshipAccessor<Self>,
        > {
            None
        }
    }
    impl Default for WaterInteractor {
        fn default() -> Self {
            Self {
                last_position: Vec3::ZERO,
                is_player: false,
                mass: 1.0,
            }
        }
    }
    fn entity_water_interaction(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        time: Res<Time>,
        mut interactors_query: Query<
            (
                Entity,
                &Transform,
                Option<&PhysicsState>,
                &mut WaterInteractor,
                Option<&Health>,
            ),
        >,
        water_query: Query<(&WaterSimData, &Transform), Without<WaterInteractor>>,
        mut params: ResMut<WaterSimParams>,
        mut impulse_writer: MessageWriter<WaterImpulseEvent>,
        water_audio: Res<crate::player::camera::WaterAudio>,
        mut local_assets: Local<
            Option<(Handle<Mesh>, Handle<StandardMaterial>, Handle<StandardMaterial>)>,
        >,
    ) {
        let (cube_mesh, foam_mat, glow_mat) = local_assets
            .get_or_insert_with(|| {
                (
                    meshes.add(Cuboid::from_size(Vec3::ONE)),
                    materials
                        .add(StandardMaterial {
                            base_color: Color::srgba(0.92, 0.97, 1.0, 0.75),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                    materials
                        .add(StandardMaterial {
                            base_color: Color::srgba(0.90, 0.96, 1.0, 0.80),
                            emissive: LinearRgba::from(Color::srgb(0.05, 0.28, 0.45))
                                .with_alpha(0.6),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                )
            })
            .clone();
        params.delta_time = time.delta_secs().min(0.03);
        params.gravity = 28.0;
        params.friction = 0.985;
        params.interactor_count = 0;
        for (entity, entity_transform, opt_physics, mut interactor, opt_health) in interactors_query
            .iter_mut()
        {
            if params.interactor_count >= 16 {
                break;
            }
            if let Some(health) = opt_health && health.hp <= 0.0 {
                continue;
            }
            let pos = entity_transform.translation;
            let last_pos = interactor.last_position;
            interactor.last_position = pos;
            let mut velocity = if time.delta_secs() > 0.0 {
                (pos - last_pos) / time.delta_secs()
            } else {
                Vec3::ZERO
            };
            let mut is_flying = false;
            if let Some(physics) = opt_physics {
                velocity = physics.velocity;
                is_flying = physics.flying;
            }
            let in_water = pos.y < 16.5;
            let was_in_water = last_pos.y < 16.5
                && interactor.last_position != Vec3::ZERO;
            let crossed_surface = in_water != was_in_water;
            let horizontal_speed = Vec2::new(velocity.x, velocity.z).length().min(12.0);
            let vertical_speed = velocity.y.clamp(-25.0, 25.0);
            let effective_mass = if interactor.is_player {
                interactor.mass
            } else {
                interactor.mass.min(0.85)
            };
            let mut data = crate::world::water_gpu::WaterInteractorData::default();
            let mut active = false;
            for (water_data, water_transform) in water_query.iter() {
                let grid_len = water_data.grid_len;
                let half_size = water_data.size * 0.5;
                let center = water_transform.translation;
                let dx = pos.x - (center.x - half_size);
                let dz = pos.z - (center.z - half_size);
                let grid_x = (dx / water_data.size) * grid_len as f32;
                let grid_z = (dz / water_data.size) * grid_len as f32;
                if grid_x >= 3.0 && grid_x < (grid_len - 3) as f32 && grid_z >= 3.0
                    && grid_z < (grid_len - 3) as f32
                {
                    if is_flying && interactor.is_player {
                        let grid_center = Vec2::new(center.x, center.z);
                        let w_height = get_water_height(
                            pos.x,
                            pos.z,
                            grid_center,
                            water_data,
                        );
                        let dist = pos.y - w_height;
                        if dist > -2.0 && dist < 25.0 {
                            let force = ((25.0 - dist.max(0.0)) / 25.0).clamp(0.0, 1.0);
                            data.grid_x = grid_x;
                            data.grid_z = grid_z;
                            data.push_force = force * 160.0
                                * (1.0 + 0.35 * (time.elapsed_secs() * 32.0).sin());
                            data.push_radius = 6.0;
                            active = true;
                            let mut rng = rand::rng();
                            let spawn_chance = 0.12 + force * 0.28;
                            if rng.random_bool(spawn_chance as f64) {
                                let particle_count = if force > 0.6 { 2 } else { 1 };
                                for _ in 0..particle_count {
                                    let angle = rng.random_range(0.0..std::f32::consts::TAU);
                                    let speed = rng.random_range(2.5..6.0) * force;
                                    let p_vel = Vec3::new(
                                        angle.cos() * speed,
                                        rng.random_range(0.8..3.5) * force,
                                        angle.sin() * speed,
                                    );
                                    let p_pos = Vec3::new(pos.x, w_height, pos.z)
                                        + Vec3::new(
                                            rng.random_range(-0.6..0.6),
                                            0.1,
                                            rng.random_range(-0.6..0.6),
                                        );
                                    commands
                                        .spawn((
                                            Mesh3d(cube_mesh.clone()),
                                            MeshMaterial3d(glow_mat.clone()),
                                            Transform::from_translation(p_pos)
                                                .with_scale(Vec3::splat(rng.random_range(0.08..0.22))),
                                            crate::player::interaction::Particle {
                                                velocity: p_vel,
                                                lifetime: Timer::from_seconds(
                                                    rng.random_range(0.5..0.9),
                                                    TimerMode::Once,
                                                ),
                                            },
                                        ));
                                }
                            }
                        }
                    } else if in_water {
                        let total_speed = (horizontal_speed + vertical_speed.abs() * 0.8)
                            .min(8.0);
                        if total_speed >= 0.15 {
                            let wave_oscillation = (time.elapsed_secs() * 12.0).sin();
                            data.grid_x = grid_x;
                            data.grid_z = grid_z;
                            data.swim_add_height = total_speed * time.delta_secs() * 2.5
                                * wave_oscillation * effective_mass;
                            data.swim_radius = 1.8 * effective_mass.powf(0.333);
                            active = true;
                            if total_speed >= 0.5 {
                                let mut rng = rand::rng();
                                let particle_chance = 0.09 * (total_speed / 2.0).min(1.0)
                                    * effective_mass.min(1.0);
                                if rng.random_bool(particle_chance as f64) {
                                    let w_height = get_water_height(
                                        pos.x,
                                        pos.z,
                                        Vec2::new(center.x, center.z),
                                        water_data,
                                    );
                                    let p_pos = Vec3::new(pos.x, w_height + 0.06, pos.z)
                                        + Vec3::new(
                                            rng.random_range(-0.5..0.5),
                                            0.0,
                                            rng.random_range(-0.5..0.5),
                                        );
                                    let p_vel = Vec3::new(
                                        rng.random_range(-1.0..1.0) * 0.4,
                                        rng.random_range(0.6..1.8) * effective_mass.min(1.0),
                                        rng.random_range(-1.0..1.0) * 0.4,
                                    );
                                    commands
                                        .spawn((
                                            Mesh3d(cube_mesh.clone()),
                                            MeshMaterial3d(foam_mat.clone()),
                                            Transform::from_translation(p_pos)
                                                .with_scale(Vec3::splat(rng.random_range(0.05..0.13))),
                                            crate::player::interaction::Particle {
                                                velocity: p_vel,
                                                lifetime: Timer::from_seconds(
                                                    rng.random_range(0.35..0.75),
                                                    TimerMode::Once,
                                                ),
                                            },
                                        ));
                                }
                            }
                        }
                    }
                    if crossed_surface && vertical_speed.abs() > 0.05
                        && interactor.last_position != Vec3::ZERO
                    {
                        let is_entering = vertical_speed < 0.0;
                        let impact_force = if is_entering {
                            (vertical_speed.abs() / 15.0).clamp(0.4, 2.0)
                                * effective_mass
                        } else {
                            (vertical_speed.abs() / 15.0).clamp(0.2, 1.2)
                                * effective_mass
                        };
                        let force = if is_entering {
                            impact_force * 160.0
                        } else {
                            -impact_force * 120.0
                        };
                        impulse_writer
                            .write(WaterImpulseEvent {
                                position: pos,
                                force,
                                radius: 5.0 * effective_mass.powf(0.333),
                            });
                        if interactor.is_player {
                            commands
                                .spawn(AudioPlayer::new(water_audio.splash_sound.clone()));
                        }
                        let mut rng = rand::rng();
                        if is_entering {
                            let particle_count = (impact_force * 32.0) as usize;
                            for _ in 0..particle_count.clamp(20, 50) {
                                let w_height = get_water_height(
                                    pos.x,
                                    pos.z,
                                    Vec2::new(center.x, center.z),
                                    water_data,
                                );
                                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                                let speed = rng.random_range(2.0..6.5) * impact_force;
                                let p_vel = Vec3::new(
                                    angle.cos() * speed,
                                    rng.random_range(3.0..8.5) * impact_force,
                                    angle.sin() * speed,
                                );
                                let p_pos = Vec3::new(pos.x, w_height, pos.z)
                                    + Vec3::new(
                                        rng.random_range(-0.6..0.6),
                                        0.1,
                                        rng.random_range(-0.6..0.6),
                                    );
                                commands
                                    .spawn((
                                        Mesh3d(cube_mesh.clone()),
                                        MeshMaterial3d(glow_mat.clone()),
                                        Transform::from_translation(p_pos)
                                            .with_scale(Vec3::splat(rng.random_range(0.08..0.18))),
                                        crate::player::interaction::Particle {
                                            velocity: p_vel,
                                            lifetime: Timer::from_seconds(
                                                rng.random_range(0.4..1.2),
                                                TimerMode::Once,
                                            ),
                                        },
                                    ));
                            }
                        } else {
                            commands
                                .queue(SafeInsertDripping {
                                    entity,
                                    dripping: Dripping {
                                        timer: Timer::from_seconds(4.2, TimerMode::Once),
                                    },
                                });
                            let particle_count = 16;
                            for _ in 0..particle_count {
                                let w_height = get_water_height(
                                    pos.x,
                                    pos.z,
                                    Vec2::new(center.x, center.z),
                                    water_data,
                                );
                                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                                let speed = rng.random_range(0.8..2.2);
                                let p_vel = Vec3::new(
                                    angle.cos() * speed,
                                    rng.random_range(-1.5..2.5),
                                    angle.sin() * speed,
                                );
                                let p_pos = Vec3::new(pos.x, w_height + 0.15, pos.z)
                                    + Vec3::new(
                                        rng.random_range(-0.4..0.4),
                                        0.0,
                                        rng.random_range(-0.4..0.4),
                                    );
                                commands
                                    .spawn((
                                        Mesh3d(cube_mesh.clone()),
                                        MeshMaterial3d(foam_mat.clone()),
                                        Transform::from_translation(p_pos)
                                            .with_scale(Vec3::splat(rng.random_range(0.05..0.11))),
                                        crate::player::interaction::Particle {
                                            velocity: p_vel,
                                            lifetime: Timer::from_seconds(
                                                rng.random_range(0.35..0.8),
                                                TimerMode::Once,
                                            ),
                                        },
                                    ));
                            }
                        }
                    }
                }
            }
            if active {
                let count = params.interactor_count as usize;
                params.interactors[count] = data;
                params.interactor_count += 1;
            }
        }
    }
    fn handle_water_l_key(
        keyboard: Res<ButtonInput<KeyCode>>,
        gamepads: Query<&Gamepad>,
        camera_query: Query<(&Camera, &GlobalTransform)>,
        windows: Query<&Window>,
        water_query: Query<(&WaterSimData, &Transform)>,
        mut params: ResMut<WaterSimParams>,
    ) {
        if params.impulse_count >= 8 {
            return;
        }
        let gamepad_l = gamepads.iter().any(|g| g.pressed(GamepadButton::LeftTrigger));
        if keyboard.pressed(KeyCode::KeyL) || gamepad_l {
            let Ok((camera, camera_transform)) = camera_query.single() else {
                return;
            };
            let Ok(window) = windows.single() else { return };
            let viewport_size = Vec2::new(window.width(), window.height());
            let center = viewport_size / 2.0;
            if let Ok(ray) = camera.viewport_to_world(camera_transform, center) {
                let t = (15.0 - ray.origin.y) / ray.direction.y;
                if t > 0.0 {
                    let hit_point = ray.origin + ray.direction * t;
                    for (water_data, transform) in water_query.iter() {
                        let grid_len = water_data.grid_len;
                        let half_size = water_data.size * 0.5;
                        let center = transform.translation;
                        let dx = hit_point.x - (center.x - half_size);
                        let dz = hit_point.z - (center.z - half_size);
                        let grid_x = (dx / water_data.size) * grid_len as f32;
                        let grid_z = (dz / water_data.size) * grid_len as f32;
                        if grid_x >= 1.0 && grid_x < (grid_len - 1) as f32
                            && grid_z >= 1.0 && grid_z < (grid_len - 1) as f32
                        {
                            let count = params.impulse_count as usize;
                            params.impulses[count] = crate::world::water_gpu::WaterImpulseData {
                                grid_x,
                                grid_z,
                                force: 12.0,
                                radius: 0.5,
                            };
                            params.impulse_count += 1;
                        }
                    }
                }
            }
        }
    }
    fn process_water_impulses(
        mut events: MessageReader<WaterImpulseEvent>,
        water_query: Query<(&WaterSimData, &Transform)>,
        mut params: ResMut<WaterSimParams>,
    ) {
        params.impulse_count = 0;
        for event in events.read() {
            if params.impulse_count >= 8 {
                break;
            }
            for (water_data, transform) in water_query.iter() {
                let grid_len = water_data.grid_len;
                let half_size = water_data.size * 0.5;
                let center = transform.translation;
                let dx = event.position.x - (center.x - half_size);
                let dz = event.position.z - (center.z - half_size);
                let grid_x = (dx / water_data.size) * grid_len as f32;
                let grid_z = (dz / water_data.size) * grid_len as f32;
                if grid_x >= 1.0 && grid_x < (grid_len - 1) as f32 && grid_z >= 1.0
                    && grid_z < (grid_len - 1) as f32
                {
                    let count = params.impulse_count as usize;
                    params.impulses[count] = crate::world::water_gpu::WaterImpulseData {
                        grid_x,
                        grid_z,
                        force: event.force,
                        radius: event.radius,
                    };
                    params.impulse_count += 1;
                }
            }
        }
    }
    pub fn get_water_height(
        world_x: f32,
        world_z: f32,
        grid_center: Vec2,
        water_sim: &WaterSimData,
    ) -> f32 {
        if world_x >= 5000.0 {
            return -1000.0;
        }
        let half_size = water_sim.size * 0.5;
        let x_offset = world_x - (grid_center.x - half_size);
        let z_offset = world_z - (grid_center.y - half_size);
        let grid_len = water_sim.grid_len;
        let grid_x_f = (x_offset / water_sim.size) * grid_len as f32;
        let grid_z_f = (z_offset / water_sim.size) * grid_len as f32;
        if grid_x_f >= 0.0 && grid_x_f < (grid_len - 1) as f32 && grid_z_f >= 0.0
            && grid_z_f < (grid_len - 1) as f32
        {
            let x0 = grid_x_f.floor() as usize;
            let x1 = x0 + 1;
            let z0 = grid_z_f.floor() as usize;
            let z1 = z0 + 1;
            let tx = grid_x_f.fract();
            let tz = grid_z_f.fract();
            let h00 = water_sim.get_height(x0, z0);
            let h10 = water_sim.get_height(x1, z0);
            let h01 = water_sim.get_height(x0, z1);
            let h11 = water_sim.get_height(x1, z1);
            let h0 = h00 * (1.0 - tx) + h10 * tx;
            let h1 = h01 * (1.0 - tx) + h11 * tx;
            15.0 + (h0 * (1.0 - tz) + h1 * tz - 1.0)
        } else {
            15.0
        }
    }
    fn apply_buoyancy(
        time: Res<Time>,
        water_query: Query<(&WaterSimData, &Transform), Without<Buoyant>>,
        mut query: Query<
            (&mut Transform, Option<&mut PhysicsState>, &Buoyant),
            (With<Buoyant>, Without<WaterSimData>),
        >,
    ) {
        let dt = time.delta_secs();
        let Ok((water_sim, water_transform)) = water_query.single() else {
            return;
        };
        let grid_center = Vec2::new(
            water_transform.translation.x,
            water_transform.translation.z,
        );
        for (mut transform, mut physics, buoyant) in query.iter_mut() {
            let pos = transform.translation;
            let w_height = get_water_height(pos.x, pos.z, grid_center, water_sim);
            if pos.y < w_height {
                if let Some(ref mut p) = physics {
                    p.velocity.y += buoyant.force * dt;
                    p.velocity.y = p.velocity.y.clamp(-1.0, 5.0);
                } else {
                    transform.translation.y += buoyant.force * 0.1 * dt;
                }
            }
        }
    }
    fn boat_ai(
        time: Res<Time>,
        water_query: Query<(&WaterSimData, &Transform), Without<Boat>>,
        mut query: Query<(&mut Transform, &Boat)>,
    ) {
        let dt = time.delta_secs();
        let Ok((water_sim, water_transform)) = water_query.single() else {
            return;
        };
        let grid_center = Vec2::new(
            water_transform.translation.x,
            water_transform.translation.z,
        );
        for (mut transform, _) in query.iter_mut() {
            let pos = transform.translation;
            let w_height = get_water_height(pos.x, pos.z, grid_center, water_sim);
            if pos.y < w_height + 0.2 {
                transform.translation.y += ((w_height + 0.1) - pos.y) * 4.0 * dt;
            } else {
                transform.translation.y -= 9.8 * dt;
            }
        }
    }
    fn update_dripping(
        mut commands: Commands,
        time: Res<Time>,
        mut dripping_query: Query<(Entity, &Transform, &mut Dripping)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut local_drip_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    ) {
        let (drip_mesh, drip_mat) = local_drip_assets
            .get_or_insert_with(|| {
                (
                    meshes.add(Cuboid::from_size(Vec3::splat(0.065))),
                    materials
                        .add(StandardMaterial {
                            base_color: Color::srgba(0.55, 0.78, 0.95, 0.65),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                )
            })
            .clone();
        let mut rng = rand::rng();
        for (entity, transform, mut dripping) in dripping_query.iter_mut() {
            dripping.timer.tick(time.delta());
            if dripping.timer.just_finished() {
                commands.entity(entity).remove::<Dripping>();
            } else {
                let spawn_count = rng.random_range(1..=2);
                for _ in 0..spawn_count {
                    let p_pos = transform.translation
                        + Vec3::new(
                            rng.random_range(-0.35..0.35),
                            rng.random_range(-0.3..1.1),
                            rng.random_range(-0.35..0.35),
                        );
                    let p_vel = Vec3::new(
                        rng.random_range(-0.1..0.1),
                        rng.random_range(-3.8..-1.8),
                        rng.random_range(-0.1..0.1),
                    );
                    commands
                        .spawn((
                            Mesh3d(drip_mesh.clone()),
                            MeshMaterial3d(drip_mat.clone()),
                            Transform::from_translation(p_pos),
                            crate::player::interaction::Particle {
                                velocity: p_vel,
                                lifetime: Timer::from_seconds(
                                    rng.random_range(0.35..0.72),
                                    TimerMode::Once,
                                ),
                            },
                        ));
                }
            }
        }
    }
}
