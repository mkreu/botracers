use std::f32::consts::PI;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::{
    image::{ImageAddressMode, ImageSampler},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const START_GRID_BOXES: usize = 12;
const START_LINE_THICKNESS: f32 = 0.4;
const START_LINE_CHECKS: usize = 24;
const GRID_FRONT_GAP: f32 = 2.0;
const GRID_BOX_WIDTH: f32 = 1.5;
const GRID_BOX_LENGTH: f32 = 0.5;
const GRID_LATERAL_OFFSET: f32 = 2.0;
const GRID_ROW_SPACING: f32 = 3.5;
const GRID_LINE_THICKNESS: f32 = 0.08;
const KERB_WIDTH: f32 = 0.5;
pub const BARRIER_WIDTH: f32 = 1.1;
pub const BARRIER_TEXTURE_REPEAT_LENGTH: f32 = 1.1;
pub const BARRIER_COLLIDER_OVERLAP: f32 = 0.2;

// TODO Fix Code reuse between here and track editor

#[derive(Resource)]
pub struct Track {
    control_points: Vec<Vec2>,
    width: f32,
    barriers: Vec<Vec<Vec2>>,
    spline: CubicCurve<Vec2>,
}

impl Track {
    pub fn new(control_points: Vec<Vec2>, width: f32, barriers: Vec<Vec<Vec2>>) -> Self {
        let spline = build_spline(&control_points);
        Self {
            control_points,
            width,
            barriers,
            spline,
        }
    }

    pub fn grid_start_position(&self, car_index: usize) -> (Vec2, f32) {
        let (start_point, tangent) =
            start_frame_from_spline(&self.spline, self.control_points[0]);
        let rotation = start_frame_rotation(tangent);
        (grid_world_position(start_point, tangent, car_index), rotation)
    }
}


pub fn setup_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
    track: Res<Track>,
) {
    let control_points = &track.control_points;
    let track_width = track.width;
    let kerb_width = KERB_WIDTH;

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(800.0, 800.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 0.6, 0.2))),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    let spline = build_spline(&control_points);

    commands.insert_resource(TrackSpline {
        spline: spline.clone(),
    });

    let track_mesh = create_track_mesh(&spline, track_width, 1000);
    commands.spawn((
        Mesh2d(meshes.add(track_mesh)),
        MeshMaterial2d(materials.add(Color::srgb(0.3, 0.3, 0.3))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let (inner_kerb, outer_kerb) = create_kerb_meshes(&spline, track_width, 1000);
    commands.spawn((
        Mesh2d(meshes.add(inner_kerb)),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        Transform::from_xyz(0.0, 0.0, 0.1),
    ));
    commands.spawn((
        Mesh2d(meshes.add(outer_kerb)),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        Transform::from_xyz(0.0, 0.0, 0.1),
    ));

    spawn_start_grid_visuals(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        track.control_points[0],
        &spline,
        track_width,
        kerb_width,
    );

    spawn_track_barriers(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        track,
    );
}

fn spawn_start_grid_visuals(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    start_node: Vec2,
    spline: &CubicCurve<Vec2>,
    track_width: f32,
    kerb_width: f32,
) {
    let (start_point, tangent) = start_frame_from_spline(spline, start_node);
    let start_rotation = start_frame_rotation(tangent);
    let line_width = (track_width - kerb_width * 2.0).max(0.0);
    let start_texture = images.add(create_start_finish_texture());
    let start_material = materials.add(ColorMaterial {
        texture: Some(start_texture),
        ..default()
    });
    commands.spawn((
        Mesh2d(meshes.add(create_start_line_mesh(line_width, START_LINE_THICKNESS))),
        MeshMaterial2d(start_material),
        Transform::from_xyz(start_point.x, start_point.y, 0.2)
            .with_rotation(Quat::from_rotation_z(start_rotation)),
    ));

    commands.spawn((
        Mesh2d(meshes.add(create_start_grid_mesh(START_GRID_BOXES))),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        Transform::from_xyz(start_point.x, start_point.y, 0.18)
            .with_rotation(Quat::from_rotation_z(start_rotation)),
    ));
}

fn start_frame_from_spline(spline: &CubicCurve<Vec2>, start_node: Vec2) -> (Vec2, Vec2) {
    const SAMPLES: usize = 1000;

    let t_max = spline.domain().end();
    let mut best_index = 0;
    let mut best_point = spline.position(0.0);
    let mut best_distance = best_point.distance_squared(start_node);

    for i in 1..SAMPLES {
        let t = (i as f32 / SAMPLES as f32) * t_max;
        let point = spline.position(t);
        let distance = point.distance_squared(start_node);
        if distance < best_distance {
            best_index = i;
            best_point = point;
            best_distance = distance;
        }
    }

    let prev_index = if best_index == 0 {
        SAMPLES - 1
    } else {
        best_index - 1
    };
    let next_index = (best_index + 1) % SAMPLES;
    let prev_t = (prev_index as f32 / SAMPLES as f32) * t_max;
    let next_t = (next_index as f32 / SAMPLES as f32) * t_max;
    let tangent = spline.position(next_t) - spline.position(prev_t);
    if tangent.length_squared() > 1e-6 {
        (best_point, tangent.normalize())
    } else {
        (best_point, Vec2::X)
    }
}

fn start_frame_rotation(tangent: Vec2) -> f32 {
    tangent.y.atan2(tangent.x) - PI / 2.0
}

fn create_start_line_mesh(width: f32, thickness: f32) -> Mesh {
    let half_width = width * 0.5;
    let half_thickness = thickness * 0.5;
    let u_repeats = START_LINE_CHECKS as f32 * 0.5;
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-half_width, -half_thickness, 0.0],
            [half_width, -half_thickness, 0.0],
            [half_width, half_thickness, 0.0],
            [-half_width, half_thickness, 0.0],
        ],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [u_repeats, 0.0], [u_repeats, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

fn create_start_grid_mesh(box_count: usize) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let color = [0.95, 0.95, 0.9, 0.75];

    for i in 0..box_count {
        let center = grid_local_position(i);
        let half_width = GRID_BOX_WIDTH * 0.5;
        let half_length = GRID_BOX_LENGTH * 0.5;
        push_colored_rect(
            &mut positions,
            &mut colors,
            &mut indices,
            center + Vec2::new(-half_width, 0.0),
            Vec2::new(GRID_LINE_THICKNESS, GRID_BOX_LENGTH),
            color,
        );
        push_colored_rect(
            &mut positions,
            &mut colors,
            &mut indices,
            center + Vec2::new(half_width, 0.0),
            Vec2::new(GRID_LINE_THICKNESS, GRID_BOX_LENGTH),
            color,
        );
        push_colored_rect(
            &mut positions,
            &mut colors,
            &mut indices,
            center + Vec2::new(0.0, half_length),
            Vec2::new(GRID_BOX_WIDTH, GRID_LINE_THICKNESS),
            color,
        );
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

fn grid_local_position(index: usize) -> Vec2 {
    let side = if index % 2 == 0 { 1.0 } else { -1.0 };
    Vec2::new(
        side * GRID_LATERAL_OFFSET,
        -(GRID_FRONT_GAP + index as f32 * GRID_ROW_SPACING),
    )
}

fn grid_world_position(start_point: Vec2, tangent: Vec2, index: usize) -> Vec2 {
    let local = grid_local_position(index) + Vec2::new(0.0, -1.5);
    let right = Vec2::new(tangent.y, -tangent.x);
    start_point + right * local.x + tangent * local.y
}

fn push_colored_rect(
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    center: Vec2,
    size: Vec2,
    color: [f32; 4],
) {
    let half_size = size * 0.5;
    let base = positions.len() as u32;
    positions.extend_from_slice(&[
        [center.x - half_size.x, center.y - half_size.y, 0.0],
        [center.x + half_size.x, center.y - half_size.y, 0.0],
        [center.x + half_size.x, center.y + half_size.y, 0.0],
        [center.x - half_size.x, center.y + half_size.y, 0.0],
    ]);
    colors.extend_from_slice(&[color; 4]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn spawn_track_barriers(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    track: Res<Track>,
) {
    let texture = images.add(create_tire_barrier_texture());
    let material = materials.add(ColorMaterial {
        texture: Some(texture),
        ..default()
    });

    for barrier in &track.barriers {
        for segment in barrier_segments(barrier) {
            commands.spawn((
                Mesh2d(meshes.add(create_textured_barrier_segment_mesh(
                    segment.length,
                    BARRIER_WIDTH,
                    BARRIER_TEXTURE_REPEAT_LENGTH,
                ))),
                MeshMaterial2d(material.clone()),
                RigidBody::Static,
                Collider::rectangle(segment.length + BARRIER_COLLIDER_OVERLAP, BARRIER_WIDTH),
                Friction::new(0.8),
                Restitution::new(0.1),
                Transform::from_xyz(segment.midpoint.x, segment.midpoint.y, 0.35)
                    .with_rotation(Quat::from_rotation_z(segment.angle)),
            ));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarrierSegment {
    pub midpoint: Vec2,
    pub length: f32,
    pub angle: f32,
}

/// The computed cubic spline for the track centre line.
#[derive(Resource)]
pub struct TrackSpline {
    pub spline: CubicCurve<Vec2>,
}

/// Build a closed cubic B-spline from control points.
pub fn build_spline(control_points: &[Vec2]) -> CubicCurve<Vec2> {
    CubicBSpline::new(control_points.to_vec())
        .to_curve_cyclic()
        .expect("Failed to create cyclic curve")
}

/// Compute the arc-length of a closed spline by sampling.
pub fn spline_length(spline: &CubicCurve<Vec2>, samples: usize) -> f32 {
    let domain = spline.domain();
    let t_max = domain.end();
    let mut length = 0.0f32;
    let mut prev = spline.position(0.0);
    for i in 1..=samples {
        let t = (i as f32 / samples as f32) * t_max;
        let p = spline.position(t);
        length += prev.distance(p);
        prev = p;
    }
    length
}

pub fn barrier_segments(points: &[Vec2]) -> Vec<BarrierSegment> {
    points
        .windows(2)
        .filter_map(|pair| {
            let start = pair[0];
            let end = pair[1];
            let delta = end - start;
            let length = delta.length();
            if length <= 1e-4 {
                return None;
            }

            Some(BarrierSegment {
                midpoint: start + delta * 0.5,
                length,
                angle: delta.y.atan2(delta.x),
            })
        })
        .collect()
}

pub fn create_textured_barrier_segment_mesh(length: f32, width: f32, repeat_length: f32) -> Mesh {
    let half_length = length * 0.5;
    let half_width = width * 0.5;
    let u_max = length / repeat_length.max(1e-4);
    let positions = vec![
        [-half_length, -half_width, 0.0],
        [half_length, -half_width, 0.0],
        [half_length, half_width, 0.0],
        [-half_length, half_width, 0.0],
    ];
    let uvs = vec![[0.0, 0.0], [u_max, 0.0], [u_max, 1.0], [0.0, 1.0]];
    let indices = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

pub fn create_tire_barrier_texture() -> Image {
    const SIZE: u32 = 32;
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - 15.5;
            let dy = y as f32 - 15.5;
            let r = (dx * dx + dy * dy).sqrt();
            let alpha = if (9.0..=15.0).contains(&r) { 255 } else { 0 };
            let shade = if (x / 4 + y / 4) % 2 == 0 { 22 } else { 36 };
            data.extend_from_slice(&[shade, shade, shade, alpha]);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image
        .sampler
        .get_or_init_descriptor()
        .set_address_mode(ImageAddressMode::Repeat);
    image.sampler = ImageSampler::Descriptor(image.sampler.get_or_init_descriptor().clone());
    image
}

pub fn create_start_finish_texture() -> Image {
    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 4;
    let mut data = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for _y in 0..HEIGHT {
        for x in 0..WIDTH {
            let shade = if x < WIDTH / 2 { 242 } else { 8 };
            data.extend_from_slice(&[shade, shade, shade, 255]);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image
        .sampler
        .get_or_init_descriptor()
        .set_address_mode(ImageAddressMode::Repeat);
    image.sampler = ImageSampler::Descriptor(image.sampler.get_or_init_descriptor().clone());
    image
}

pub fn create_track_mesh(spline: &CubicCurve<Vec2>, track_width: f32, segments: usize) -> Mesh {
    let domain = spline.domain();
    let t_max = domain.end();

    let mut positions = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices along the spline
    for i in 0..segments {
        let t1 = (i as f32 / segments as f32) * t_max;
        let t2 = (((i + 1) % segments) as f32 / segments as f32) * t_max;

        let p1 = spline.position(t1);
        let p2 = spline.position(t2);

        // Calculate perpendicular direction for track width
        let tangent = (p2 - p1).normalize();
        let normal = vec2(-tangent.y, tangent.x);

        // Inner and outer edge vertices
        let inner = p1 - normal * track_width * 0.5;
        let outer = p1 + normal * track_width * 0.5;

        positions.push([inner.x, inner.y, 0.0]);
        positions.push([outer.x, outer.y, 0.0]);
    }

    // Generate triangle indices
    for i in 0..segments {
        let base = (i * 2) as u32;
        let next_base = ((i + 1) % segments * 2) as u32;

        // Two triangles per segment
        indices.push(base);
        indices.push(next_base);
        indices.push(base + 1);

        indices.push(base + 1);
        indices.push(next_base);
        indices.push(next_base + 1);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

pub fn create_kerb_meshes(
    spline: &CubicCurve<Vec2>,
    track_width: f32,
    segments: usize,
) -> (Mesh, Mesh) {
    let domain = spline.domain();
    let t_max = domain.end();
    let kerb_stripe_length = 1; // Number of segments per stripe color

    let mut inner_positions = Vec::new();
    let mut inner_colors = Vec::new();
    let mut inner_indices = Vec::new();

    let mut outer_positions = Vec::new();
    let mut outer_colors = Vec::new();
    let mut outer_indices = Vec::new();

    // Generate vertices - 4 vertices per segment (not shared with adjacent segments)
    for i in 0..segments {
        let t = (i as f32 / segments as f32) * t_max;
        let t_next = (((i + 1) % segments) as f32 / segments as f32) * t_max;

        let p = spline.position(t);
        let p_next = spline.position(t_next);

        // Calculate normals at both the start and end of this segment
        let i_prev = if i == 0 { segments - 1 } else { i - 1 };
        let t_prev = (i_prev as f32 / segments as f32) * t_max;
        let p_prev = spline.position(t_prev);
        let t_after = (((i + 2) % segments) as f32 / segments as f32) * t_max;
        let p_after = spline.position(t_after);

        // Normal at start: perpendicular to direction from prev to current
        let tangent_start = (p_next - p_prev).normalize();
        let normal_start = vec2(-tangent_start.y, tangent_start.x);

        // Normal at end: perpendicular to direction from current to after
        let tangent_end = (p_after - p).normalize();
        let normal_end = vec2(-tangent_end.y, tangent_end.x);

        // Determine color for this segment
        let is_red = (i / kerb_stripe_length) % 2 == 0;
        let color = if is_red {
            [0.9, 0.1, 0.1, 1.0]
        } else {
            [0.95, 0.95, 0.95, 1.0]
        };

        // Inner kerb - use appropriate normal at each end
        let inner_edge_start = p - normal_start * track_width * 0.5;
        let inner_outer_start = p - normal_start * (track_width * 0.5 - KERB_WIDTH);
        let inner_edge_end = p_next - normal_end * track_width * 0.5;
        let inner_outer_end = p_next - normal_end * (track_width * 0.5 - KERB_WIDTH);

        let base_idx = inner_positions.len() as u32;
        inner_positions.push([inner_edge_start.x, inner_edge_start.y, 0.0]);
        inner_positions.push([inner_outer_start.x, inner_outer_start.y, 0.0]);
        inner_positions.push([inner_edge_end.x, inner_edge_end.y, 0.0]);
        inner_positions.push([inner_outer_end.x, inner_outer_end.y, 0.0]);

        // All 4 vertices get the same color for sharp transition
        inner_colors.push(color);
        inner_colors.push(color);
        inner_colors.push(color);
        inner_colors.push(color);

        // Two triangles for this segment
        inner_indices.push(base_idx);
        inner_indices.push(base_idx + 2);
        inner_indices.push(base_idx + 1);

        inner_indices.push(base_idx + 1);
        inner_indices.push(base_idx + 2);
        inner_indices.push(base_idx + 3);

        // Outer kerb - use appropriate normal at each end
        let outer_inner_start = p + normal_start * (track_width * 0.5 - KERB_WIDTH);
        let outer_edge_start = p + normal_start * track_width * 0.5;
        let outer_inner_end = p_next + normal_end * (track_width * 0.5 - KERB_WIDTH);
        let outer_edge_end = p_next + normal_end * track_width * 0.5;

        let base_idx = outer_positions.len() as u32;
        outer_positions.push([outer_inner_start.x, outer_inner_start.y, 0.0]);
        outer_positions.push([outer_edge_start.x, outer_edge_start.y, 0.0]);
        outer_positions.push([outer_inner_end.x, outer_inner_end.y, 0.0]);
        outer_positions.push([outer_edge_end.x, outer_edge_end.y, 0.0]);

        outer_colors.push(color);
        outer_colors.push(color);
        outer_colors.push(color);
        outer_colors.push(color);

        outer_indices.push(base_idx);
        outer_indices.push(base_idx + 2);
        outer_indices.push(base_idx + 1);

        outer_indices.push(base_idx + 1);
        outer_indices.push(base_idx + 2);
        outer_indices.push(base_idx + 3);
    }

    let mut inner_mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    inner_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, inner_positions);
    inner_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, inner_colors);
    inner_mesh.insert_indices(bevy::mesh::Indices::U32(inner_indices));

    let mut outer_mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    outer_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, outer_positions);
    outer_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, outer_colors);
    outer_mesh.insert_indices(bevy::mesh::Indices::U32(outer_indices));

    (inner_mesh, outer_mesh)
}

/// Sample inner and outer track borders as closed polylines.
///
/// This uses spline tangents with neighboring samples to compute a stable normal,
/// then offsets by half the track width on both sides.
pub fn sample_track_borders(
    spline: &CubicCurve<Vec2>,
    track_width: f32,
    segments: usize,
) -> (Vec<Vec2>, Vec<Vec2>) {
    let domain = spline.domain();
    let t_max = domain.end();
    let mut inner = Vec::with_capacity(segments);
    let mut outer = Vec::with_capacity(segments);

    for i in 0..segments {
        let i_prev = if i == 0 { segments - 1 } else { i - 1 };
        let i_next = (i + 1) % segments;

        let t_prev = (i_prev as f32 / segments as f32) * t_max;
        let t = (i as f32 / segments as f32) * t_max;
        let t_next = (i_next as f32 / segments as f32) * t_max;

        let p_prev = spline.position(t_prev);
        let p = spline.position(t);
        let p_next = spline.position(t_next);

        let tangent = (p_next - p_prev).normalize();
        let normal = vec2(-tangent.y, tangent.x);

        inner.push(p - normal * track_width * 0.5);
        outer.push(p + normal * track_width * 0.5);
    }

    (inner, outer)
}
