use bevy::prelude::*;
use bevy_animation_graph::core::animation_graph::{AnimationGraph, GraphInputPin};
use bevy_animation_graph::core::animation_graph_player::{AnimationGraphPlayer, AnimationSource};
use bevy_animation_graph::core::animation_node::{NodeLike, AnimationNode};
use bevy_animation_graph::core::context::spec_context::SpecContext;
use bevy_animation_graph::core::context::new_context::NodeContext;
use bevy_animation_graph::core::edge_data::{DataSpec, DataValue};
use bevy_animation_graph::core::animation_clip::EntityPath;
use bevy_animation_graph::core::pose::{Pose, BonePose};
use bevy_animation_graph::core::errors::GraphError;
use bevy_rapier3d::prelude::*;
use crate::player::camera::{RagdollLimb, RagdollJoint};

#[derive(Reflect, Clone, Debug, Default)]
#[reflect(Default)]
pub struct VoxelWalkNode;

impl NodeLike for VoxelWalkNode {
    fn display_name(&self) -> String {
        "Voxel Walk".to_string()
    }

    fn spec(&self, mut ctx: SpecContext<'_>) -> Result<(), GraphError> {
        ctx.add_input_data("time", DataSpec::F32);
        ctx.add_input_data("speed", DataSpec::F32);
        ctx.add_output_data("pose", DataSpec::Pose);
        Ok(())
    }

    fn update(&self, mut ctx: NodeContext<'_>) -> Result<(), GraphError> {
        let time = ctx.data_back("time")?.as_f32().unwrap_or(0.0);
        let speed = ctx.data_back("speed")?.as_f32().unwrap_or(1.0);
        
        let mut pose = Pose::default();
        
        // Simple procedural walk cycle logic
        let cycle = (time * speed * 5.0).sin();
        let amplitude = 0.8; // Increased amplitude
        
        // Pelvis bounce/rotation
        pose.add_bone(BonePose {
            rotation: Some(Quat::from_rotation_x(cycle * 0.15)),
            translation: Some(Vec3::new(0.0, cycle.abs() * 0.08, 0.0)),
            ..Default::default()
        }, EntityPath::from_slashed_string("Pelvis".to_string()).id());

        // Left leg
        pose.add_bone(BonePose {
            rotation: Some(Quat::from_rotation_x(cycle * amplitude)),
            ..Default::default()
        }, EntityPath::from_slashed_string("HipLeft".to_string()).id());
        
        pose.add_bone(BonePose {
            rotation: Some(Quat::from_rotation_x((cycle - 0.5).max(0.0) * amplitude * 1.5)),
            ..Default::default()
        }, EntityPath::from_slashed_string("KneeLeft".to_string()).id());

        // Right leg
        pose.add_bone(BonePose {
            rotation: Some(Quat::from_rotation_x(-cycle * amplitude)),
            ..Default::default()
        }, EntityPath::from_slashed_string("HipRight".to_string()).id());
        
        pose.add_bone(BonePose {
            rotation: Some(Quat::from_rotation_x((-cycle - 0.5).max(0.0) * amplitude * 1.5)),
            ..Default::default()
        }, EntityPath::from_slashed_string("KneeRight".to_string()).id());

        ctx.set_data_fwd("pose", DataValue::from(pose));
        
        Ok(())
    }

    fn duration(&self, _ctx: NodeContext<'_>) -> Result<(), GraphError> {
        Ok(())
    }
}

pub struct AnimationSystemPlugin;

impl Plugin for AnimationSystemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<VoxelWalkNode>()
            .add_systems(Update, (
                setup_advanced_animation_graph, 
                update_animation_time,
                apply_animation_to_ragdoll
            ));
    }
}

pub fn setup_advanced_animation_graph(
    mut _commands: Commands,
    mut query: Query<(Entity, &mut AnimationGraphPlayer), Added<AnimationGraphPlayer>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (_entity, mut player) in query.iter_mut() {
        let mut graph = AnimationGraph::default();
        
        // Create nodes
        let walk_node = AnimationNode::new("Walk", VoxelWalkNode);
        let walk_node_id = walk_node.id;
        graph.add_node(walk_node);
        
        // Connect to graph output
        graph.add_output_data("pose".to_string(), DataSpec::Pose);
        graph.add_output_data_edge(walk_node_id, "pose", "pose");
        
        // Expose graph inputs
        let time_pin = GraphInputPin::Passthrough("time".to_string());
        graph.add_input_data(time_pin.clone(), DataSpec::F32);
        graph.add_input_data_edge(time_pin, walk_node_id, "time");

        let speed_pin = GraphInputPin::Passthrough("speed".to_string());
        graph.add_input_data(speed_pin.clone(), DataSpec::F32);
        graph.add_input_data_edge(speed_pin, walk_node_id, "speed");
        
        let graph_handle = graphs.add(graph);
        player.start(graph_handle);
        
        // Set default inputs
        player.set_input_data("speed", DataValue::F32(1.0));
        player.set_input_data("time", DataValue::F32(0.0));
    }
}

pub fn update_animation_time(
    time: Res<Time>,
    mut query: Query<&mut AnimationGraphPlayer>,
) {
    for mut player in query.iter_mut() {
        player.set_input_data("time", DataValue::F32(time.elapsed_secs()));
    }
}

pub fn apply_animation_to_ragdoll(
    query: Query<(&AnimationGraphPlayer, &Children)>,
    mut limb_query: Query<(&RagdollJoint, &mut MultibodyJoint)>,
) {
    for (player, children) in query.iter() {
        let outputs = player.get_outputs();
        
        let Some(data_value) = outputs.get(&"pose".to_string()) else { 
            continue; 
        };
        let Ok(pose) = data_value.as_pose() else { 
            continue; 
        };
        
        for child in children.iter() {
            if let Ok((joint_info, mut joint)) = limb_query.get_mut(child) {
                let side_suffix = if joint_info.side > 0.0 { "Right" } else { "Left" };
                let bone_name = match joint_info.limb {
                    RagdollLimb::Hip => format!("Hip{}", side_suffix),
                    RagdollLimb::Knee => format!("Knee{}", side_suffix),
                    RagdollLimb::Shoulder => format!("Shoulder{}", side_suffix),
                    RagdollLimb::Elbow => format!("Elbow{}", side_suffix),
                    RagdollLimb::Torso => "Pelvis".to_string(), // Fixed: mapped to Pelvis
                };

                let bone_id = EntityPath::from_slashed_string(bone_name).id();
                if let Some(bone_pose) = pose.get_bone(bone_id) {
                    // Stiffer motors for weight bearing
                    let stiffness = 20000.0;
                    let damping = 1000.0;
                    
                    match &mut joint.data {
                        TypedJoint::SphericalJoint(s) => {
                            if let Some(rot) = bone_pose.rotation {
                                let (y, x, z) = rot.to_euler(EulerRot::YXZ);
                                s.set_motor_position(JointAxis::AngX, x, stiffness, damping);
                                s.set_motor_position(JointAxis::AngY, y, stiffness, damping);
                                s.set_motor_position(JointAxis::AngZ, z, stiffness, damping);
                            }
                        }
                        TypedJoint::RevoluteJoint(r) => {
                            if let Some(rot) = bone_pose.rotation {
                                let (_, x, _) = rot.to_euler(EulerRot::YXZ);
                                r.set_motor_position(x, stiffness, damping);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
