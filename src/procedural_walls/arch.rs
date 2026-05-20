//! Procedural arch generation for the brick wall system.
//!
//! Generates semicircular voussoir arches above gateways and across close
//! wall segment endpoints. Each voussoir brick is identical in structure to
//! regular `ProceduralBrick` entities so mining, health, and physics come for
//! free with no extra code.
//!
//! # Arch geometry
//! A semicircular arch is defined by its two **impost** (foot) positions in
//! world space. The keystone sits at the top of the semicircle:
//!
//! ```text
//!        keystone
//!       /   |   \
//!      /    |    \
//!  left     |    right
//!  impost  rise  impost
//!  |<-- span -->|
//! ```
//!
//! `rise = span / 2` (true semicircle).  Voussoirs are evenly spaced by arc
//! angle, each rotated so its local Y axis points radially outward.

use std::f32::consts::PI;
use bevy::prelude::{Transform, Vec2, Vec3};
use bevy::math::{Mat3, Quat};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Arc length consumed by each voussoir brick (in metres).
/// Smaller = more bricks, finer arch. Matches wall `BRICK_HEIGHT` (0.22 m).
const VOUSSOIR_ARC_WIDTH: f32 = 0.22;

/// Radial thickness of each voussoir (how tall the brick is, pointing inward).
pub const VOUSSOIR_RADIAL_HEIGHT: f32 = 0.30;

/// Maximum horizontal span before we stop generating an arch.
pub const MAX_ARCH_SPAN: f32 = 4.0;

/// Minimum horizontal span to bother with an arch (avoid tiny arches).
pub const MIN_ARCH_SPAN: f32 = 0.6;

/// Depth of arch bricks along the wall thickness axis (matches wall brick depth).
const ARCH_DEPTH: f32 = 0.35;

/// Maximum Y difference between two wall-end tops before they're considered
/// "mismatched" and no auto-arch is generated.
pub const MAX_HEIGHT_MISMATCH: f32 = 0.5;

/// Maximum horizontal distance between two wall-segment endpoints that triggers
/// auto-arch bridging.
pub const AUTO_ARCH_ENDPOINT_DIST: f32 = 3.5;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single voussoir brick ready to be spawned into the world.
#[derive(Clone)]
pub struct ArchBrick {
    /// World-space transform: translation at voussoir centroid,
    /// rotation so +Y is radially outward, scale = (arc_width, radial_height, depth).
    pub transform: Transform,
    /// Normalised arc position [0 = left impost, 1 = right impost].
    /// Used to compute staggered spawn animation delays.
    pub arc_t: f32,
}

/// Describes an opening that wants an arch (used for both gateway carves and
/// auto-detected endpoint gaps).
#[derive(Clone, Debug)]
pub struct ArchOpening {
    /// World position of the left impost (base of arch, left side).
    pub left_foot: Vec3,
    /// World position of the right impost (base of arch, right side).
    pub right_foot: Vec3,
}

// ---------------------------------------------------------------------------
// Core generator
// ---------------------------------------------------------------------------

/// Generates the voussoir bricks for a semicircular arch over `opening`.
///
/// Returns an empty `Vec` if the span is outside [`MIN_ARCH_SPAN`] …
/// [`MAX_ARCH_SPAN`].
pub fn generate_arch(opening: &ArchOpening) -> Vec<ArchBrick> {
    let left  = opening.left_foot;
    let right = opening.right_foot;

    // Horizontal span vector (ignore Y for span calculation)
    let span_vec_xz = Vec3::new(right.x - left.x, 0.0, right.z - left.z);
    let span = span_vec_xz.length();

    if span < MIN_ARCH_SPAN || span > MAX_ARCH_SPAN {
        return Vec::new();
    }

    let radius = span / 2.0;

    // Arch centre sits at the midpoint horizontally, resting flush on the imposts.
    let mid_foot = (left + right) * 0.5;
    let foot_y   = left.y.max(right.y); // use the higher of the two impost heights
    let center   = Vec3::new(mid_foot.x, foot_y, mid_foot.z);

    // Normalised horizontal direction from left to right impost.
    let span_dir = span_vec_xz.normalize();
    // Wall-normal (perpendicular to span, horizontal).
    let wall_normal = Vec3::new(-span_dir.z, 0.0, span_dir.x);

    // How many voussoirs fit on the semicircle?
    let arc_length    = PI * radius; // half circumference
    let num_voussoirs = ((arc_length / VOUSSOIR_ARC_WIDTH).round() as usize).max(3);

    let mut bricks = Vec::with_capacity(num_voussoirs);

    for i in 0..num_voussoirs {
        // arc_t goes 0 → 1 from left impost to right impost.
        // Angle goes from PI (left) → 0 (right) so that Y is upward.
        let arc_t_mid = (i as f32 + 0.5) / num_voussoirs as f32;
        let angle_mid = PI * (1.0 - arc_t_mid);

        // arc_t for the two edges (used for scale calculation)
        let arc_t_lo  = i as f32 / num_voussoirs as f32;
        let arc_t_hi  = (i as f32 + 1.0) / num_voussoirs as f32;
        let angle_lo  = PI * (1.0 - arc_t_lo);
        let angle_hi  = PI * (1.0 - arc_t_hi);

        // Centroid position on the arch centreline.
        // In the arch's local 2-D frame: X along span, Y vertical.
        let pos_local_mid = Vec2::new(angle_mid.cos() * radius, angle_mid.sin() * radius);

        // Centroid in world space.
        let centroid = center
            + span_dir   * pos_local_mid.x
            + Vec3::Y    * pos_local_mid.y;

        // Radial outward direction — computed for reference, used to derive local_y
        let _radial_out = (centroid - center).normalize();

        // Tangent along the arch at this voussoir (perpendicular to radial in the arch plane).
        // Points from left-impost side toward right-impost side.
        let tangent_local_lo = Vec2::new(angle_lo.cos(), angle_lo.sin());
        let tangent_local_hi = Vec2::new(angle_hi.cos(), angle_hi.sin());
        let tang_mid_local   = (tangent_local_hi - tangent_local_lo).normalize();
        // The tangent in world space (rotated into span_dir / Y plane):
        let arc_tangent = span_dir * tang_mid_local.x + Vec3::Y * tang_mid_local.y;

        // Approximate arc-width at this voussoir (chord between its two edge positions).
        let pos_lo = center + span_dir * (angle_lo.cos() * radius) + Vec3::Y * (angle_lo.sin() * radius);
        let pos_hi = center + span_dir * (angle_hi.cos() * radius) + Vec3::Y * (angle_hi.sin() * radius);
        let actual_arc_width = pos_lo.distance(pos_hi).max(0.05);

        // Build rotation: local X = along arc tangent, local Y = radially outward,
        // local Z = along wall normal (depth direction).
        //   We want the brick face to be visible from the arch interior,
        //   so local Z points in wall_normal direction.
        let local_x = arc_tangent.normalize();
        let local_z = wall_normal;
        let local_y = local_z.cross(local_x).normalize(); // ensures right-hand system; close to radial_out

        // Safety: if any are degenerate, skip this voussoir.
        if local_x.length_squared() < 0.5 || local_y.length_squared() < 0.5 {
            continue;
        }

        let rotation = Quat::from_mat3(&Mat3::from_cols(local_x, local_y, local_z));

        let scale = Vec3::new(actual_arc_width, VOUSSOIR_RADIAL_HEIGHT, ARCH_DEPTH);

        bricks.push(ArchBrick {
            transform: Transform {
                translation: centroid,
                rotation,
                scale,
            },
            arc_t: arc_t_mid,
        });
    }

    bricks
}

// ---------------------------------------------------------------------------
// Spawn-delay helper
// ---------------------------------------------------------------------------

/// Returns a staggered spawn delay for a voussoir at `arc_t` [0..1].
/// Feet (arc_t near 0 or 1) appear first; keystone (arc_t = 0.5) appears last.
/// Total spread: ~0.5 s.
pub fn voussoir_spawn_delay(arc_t: f32) -> f32 {
    // Delay is proportional to how close to the keystone the brick is.
    // |arc_t - 0.5| is 0.5 at the feet and 0.0 at the keystone.
    let closeness_to_keystone = 1.0 - (arc_t - 0.5).abs() * 2.0; // 0 at feet, 1 at keystone
    closeness_to_keystone * 0.55
}

// ---------------------------------------------------------------------------
// Auto-arch gap detection
// ---------------------------------------------------------------------------

/// Data about one end of a wall column — used by the auto-arch detector.
#[derive(Clone, Debug)]
pub struct WallEndpoint {
    /// World position of the top-centre of this wall end.
    pub top_center: Vec3,
    /// World position of the bottom-centre (ground level) of this wall end.
    pub bottom_center: Vec3,
    /// Whether this is the left (false) or right (true) end of the column.
    pub is_right_end: bool,
}

/// Given a list of all current wall top-end positions, return pairs that should
/// receive an auto-generated arch.  The pairs are sorted so `left` is always
/// the one with lower X+Z centroid value (consistent ordering avoids duplicates).
pub fn find_arch_openings(endpoints: &[WallEndpoint]) -> Vec<ArchOpening> {
    let mut openings = Vec::new();

    for (i, a) in endpoints.iter().enumerate() {
        for b in endpoints.iter().skip(i + 1) {
            let a_xz = Vec2::new(a.top_center.x, a.top_center.z);
            let b_xz = Vec2::new(b.top_center.x, b.top_center.z);
            let dist = a_xz.distance(b_xz);

            // Must be within bridging range
            if dist < MIN_ARCH_SPAN || dist > AUTO_ARCH_ENDPOINT_DIST {
                continue;
            }

            // Heights should be similar
            let height_diff = (a.top_center.y - b.top_center.y).abs();
            if height_diff > MAX_HEIGHT_MISMATCH {
                continue;
            }

            // Order left-to-right by X primarily, then Z
            let (left_ep, right_ep) = if a_xz.x < b_xz.x
                || (a_xz.x == b_xz.x && a_xz.y < b_xz.y)
            {
                (a, b)
            } else {
                (b, a)
            };

            openings.push(ArchOpening {
                left_foot:  left_ep.top_center,
                right_foot: right_ep.top_center,
            });
        }
    }

    openings
}
