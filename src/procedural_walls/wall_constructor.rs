//! Wall constructor for procedural brick wall generation along curves.
//!
//! This module implements procedural brick wall generation using seeded randomization.
//! Bricks are positioned along a curve path with randomized dimensions and optional
//! horizontal splitting for realistic brick patterns.

use super::brick::Brick;
use super::curve::Curve;
use bevy::math::{Mat3, Quat};
use bevy::prelude::{Transform, Vec2, Vec3};
use fastrand::Rng;

// High-fidelity rectangular chiseled dimensions (approx 2.5:1 ratio)
const BRICK_WIDTH: f32 = 0.55;
const BRICK_WIDTH_VARIANCE: f32 = 0.15;

const BRICK_HEIGHT: f32 = 0.22;
const BRICK_HEIGHT_VARIANCE: f32 = 0.05;

const BRICK_DEPTH: f32 = 0.35;
const BRICK_DEPTH_VARIANCE: f32 = 0.08;

pub struct WallConstructor;

impl WallConstructor {
    pub fn from_curve(
        curve: &Curve,
        wall_height: f32,
        get_ground_y: impl Fn(Vec3) -> f32,
    ) -> Vec<Brick> {
        let mut rng = fastrand::Rng::with_seed(0);

        let wall_length: f32 = curve.length;

        // Calculate curve span and vertical top in world space using true ground heights
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for p in &curve.points {
            let gy = get_ground_y(*p);
            if gy < min_y {
                min_y = gy;
            }
            if p.y > max_y {
                max_y = p.y;
            }
        }
        let top_y = max_y + wall_height;
        let max_span = (top_y - min_y).max(wall_height);

        // Calculate rows globally from the flat top down to the lowest ground
        let max_row_count = (max_span / BRICK_HEIGHT).ceil().max(1.0) as usize;

        let rows = random_splits(max_row_count, BRICK_HEIGHT_VARIANCE / max_span, &mut rng);
        let bricks_per_row = (wall_length / BRICK_WIDTH).ceil().max(1.0) as usize;

        let mut bricks = Vec::new();
        for r in 0..max_row_count {
            let row_u = rows[r];

            // Stagger alternate rows (running bond pattern!)
            let is_odd = r % 2 == 1;
            let mut split_points = Vec::new();
            if is_odd {
                split_points.push(0.0);
                for k in 1..=bricks_per_row {
                    let u = (k as f32 - 0.5) / (bricks_per_row as f32);
                    if u < 1.0 {
                        split_points.push(u);
                    }
                }
                split_points.push(1.0);
            } else {
                split_points = (0..=bricks_per_row)
                    .map(|k| k as f32 / bricks_per_row as f32)
                    .collect();
            }

            // Perturb splits slightly for organic variation (running bond with hand-built look!)
            let brick_widths =
                perturb_splits(&split_points, BRICK_WIDTH_VARIANCE / wall_length, &mut rng);

            let brick_height = if let Some(&next_row_u) = rows.get(r + 1) {
                (next_row_u - row_u) * max_span
            } else {
                BRICK_HEIGHT + (rng.f32() - 0.5) * BRICK_HEIGHT_VARIANCE
            };

            let mut brick_row: Vec<Brick> = Vec::new();
            for j in 0..brick_widths.len() {
                if let Some(&next_u) = brick_widths.get(j + 1) {
                    let this_u = brick_widths[j];

                    // Skip some top-most row bricks for weathered crenellation/castle effect
                    if r == 0 && rng.f32() < 0.35 {
                        continue;
                    }

                    let brick_depth = BRICK_DEPTH + (rng.f32() - 0.5) * BRICK_DEPTH_VARIANCE;

                    // Vertically split brick chance (except top row) for stone masonry variety
                    if rng.f32() < 0.4 && r != 0 {
                        let range = (0.3, 0.7);
                        let random_split = rng.f32() * (range.1 - range.0) + range.0;
                        let pivot_u = ((next_u + this_u) / 2.0).clamp(0.0, 1.0);

                        let height_u_1 = brick_height / max_span * random_split;
                        let height_u_2 = brick_height / max_span * (1.0 - random_split);

                        let pivot_v_1 = row_u + height_u_1 / 2.0;
                        let pivot_v_2 = (row_u + brick_height / max_span) - height_u_2 / 2.0;

                        let width_u = next_u - this_u;
                        let width_ws = width_u * wall_length;

                        for (height, pivot_v) in [(height_u_1, pivot_v_1), (height_u_2, pivot_v_2)]
                        {
                            let brick_center_y = top_y - pivot_v * max_span;
                            let curve_pos = curve.get_pos_at_u(pivot_u);
                            let ground_y = get_ground_y(curve_pos);

                            // Skip if entirely below ground
                            let half_height = (height * max_span) / 2.0;
                            if brick_center_y + half_height < ground_y {
                                continue;
                            }

                            brick_row.push(Brick {
                                pivot_uv: Vec2::new(pivot_u, pivot_v),
                                bounds_uv: Vec2::new(width_u, height),
                                transform: Transform {
                                    translation: Vec3::new(pivot_u * wall_length, 0.0, 0.0),
                                    rotation: Quat::IDENTITY,
                                    scale: Vec3::new(width_ws, height * max_span, brick_depth),
                                },
                            });
                        }
                    } else {
                        let pivot_u = ((next_u + this_u) / 2.0).clamp(0.0, 1.0);
                        let width_u = next_u - this_u;
                        let width_ws = width_u * wall_length;
                        let pivot_v = row_u + brick_height / max_span / 2.0;

                        let brick_center_y = top_y - pivot_v * max_span;
                        let curve_pos = curve.get_pos_at_u(pivot_u);
                        let ground_y = get_ground_y(curve_pos);

                        // Skip if entirely below ground
                        let half_height = brick_height / 2.0;
                        if brick_center_y + half_height < ground_y {
                            continue;
                        }

                        brick_row.push(Brick {
                            pivot_uv: Vec2::new(pivot_u, pivot_v),
                            bounds_uv: Vec2::new(width_u, brick_height / max_span),
                            transform: Transform {
                                scale: Vec3::new(width_ws, brick_height, brick_depth),
                                translation: Vec3::new(pivot_u * wall_length, 0.0, 0.0),
                                rotation: Quat::IDENTITY,
                            },
                        });
                    }
                }
            }

            // Transform bricks into world space
            for brick in &mut brick_row {
                let curve_pos = curve.get_pos_at_u(brick.pivot_uv.x);
                brick.transform.translation = curve_pos;
                // Calculate absolute vertical translation
                brick.transform.translation.y = top_y - brick.pivot_uv.y * max_span;

                let curve_tangent = curve.get_tangent_at_u(brick.pivot_uv.x);
                let normal = curve_tangent.cross(Vec3::Y);
                brick.transform.rotation =
                    Quat::from_mat3(&Mat3::from_cols(curve_tangent, Vec3::Y, normal));
            }

            bricks.extend(brick_row);
        }

        bricks
    }
}

/// Generate random splits in [0;1] range with variance perturbation.
fn random_splits(splits: usize, variance_u: f32, rng: &mut Rng) -> Vec<f32> {
    let row_u: Vec<f32> = (0..(splits + 1))
        .map(|i| (i as f32) / (splits as f32))
        .collect();

    row_u
        .iter()
        .enumerate()
        .map(|(i, u)| {
            if i != 0 && i != row_u.len() - 1 {
                (u + (rng.f32() - 0.5) * variance_u).clamp(0.0, 1.0)
            } else {
                *u
            }
        })
        .collect()
}

/// Perturbs the staggered split points along a row for organic hand-masonry variety.
fn perturb_splits(splits: &[f32], variance_u: f32, rng: &mut Rng) -> Vec<f32> {
    splits
        .iter()
        .enumerate()
        .map(|(i, &u)| {
            if i != 0 && i != splits.len() - 1 {
                (u + (rng.f32() - 0.5) * variance_u).clamp(0.0, 1.0)
            } else {
                u
            }
        })
        .collect()
}
