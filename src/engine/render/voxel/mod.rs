mod dag;

use bevy::{prelude::*,
           render::{mesh::Indices,
                    pipeline::{PipelineDescriptor, PrimitiveTopology,
                               RenderPipeline},
                    shader::{ShaderStage, ShaderStages}}};
use building_blocks::mesh::{greedy_quads, GreedyQuadsBuffer, OrientedCubeFace,
                            UnorientedQuad, RIGHT_HANDED_Y_UP_CONFIG};
use building_blocks::prelude::*;
use building_blocks::storage::Channel;

use crate::engine::asset::{dot_vox_loader::{VoxModel, WorldVoxel},
                           BuildingAssets, TextureAssets};

pub fn build(
    // Builds the world
    mut commands: Commands,
    texture_handle: Res<TextureAssets>,
    building_handles: Res<BuildingAssets>,
    mut models: ResMut<Assets<VoxModel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    // Draw terrain
    let world_extent =
        Extent3i::from_min_and_shape(PointN([0; 3]), PointN([40, 100, 40]));
    let mut world_array = Array3x1::fill(world_extent, WorldVoxel::EMPTY);
    let base =
        Extent3i::from_min_and_shape(PointN([0, 0, 0]), PointN([40, 2, 40]));
    let base_slope =
        Extent3i::from_min_and_shape(PointN([0, 2, 0]), PointN([20, 2, 40]));
    let left_edge =
        Extent3i::from_min_and_shape(PointN([0, 2, 0]), PointN([40, 5, 10]));
    let right_edge =
        Extent3i::from_min_and_shape(PointN([0, 2, 30]), PointN([40, 5, 10]));
    world_array.fill_extent(&base, WorldVoxel(1));
    world_array.fill_extent(&left_edge, WorldVoxel(1));
    world_array.fill_extent(&right_edge, WorldVoxel(1));
    world_array.fill_extent(&base_slope, WorldVoxel(1));

    let mut surface_nets_buffer =
        building_blocks::mesh::surface_nets::SurfaceNetsBuffer::default();
    let world_sdf = boolean_sdf(world_extent, &world_array);
    let averaged_world_sdf = averaged_sdf(world_extent, &world_sdf, 1);

    building_blocks::mesh::surface_nets::surface_nets(
        &averaged_world_sdf,
        &world_extent,
        2.0,
        &mut surface_nets_buffer,
    );

    let water_extent =
        Extent3i::from_min_and_shape(PointN([5, 5, 40]), PointN([1, 1, 1]));
    let mut water_array = Array3x1::fill(water_extent, WorldVoxel(1));
    let water_mesh =
        water_array.for_each(&water_extent, |p: Point3i, voxel: WorldVoxel| {
            if voxel == WorldVoxel(1) {
                let x = p.x() as f32;
                let y = p.y() as f32;
                let z = p.z() as f32;
                let back_left = [x, y, z];
                let back_right = [x + 2.0, y, z];
                let front_left = [x, y, z + 2.0];
                let front_right = [x + 2.0, y, z + 2.0];
                let top_center = [x + 1.0, y + 2.0, z + 1.0];

                let vertices = [
                    // Base square with a normal pointing down
                    (back_left, [0.0, -1.0, 0.0], [0.0, 0.0]),
                    (back_right, [0.0, -1.0, 0.0], [1.0, 0.0]),
                    (front_right, [0.0, -1.0, 0.0], [0.0, 1.0]),
                    (front_left, [0.0, -1.0, 0.0], [1.0, 1.0]),
                    // Triangle on the left side connecting to the top middle
                    (back_left, [-1.0, 1.0, 0.0], [0.0, 0.0]),
                    (top_center, [-1.0, 1.0, 0.0], [0.0, 0.0]),
                    (front_left, [-1.0, 1.0, 0.0], [0.0, 0.0]),
                    // Triangle on the front side connecting to the top middle
                    (front_left, [0.0, 1.0, 1.0], [0.0, 0.0]),
                    (top_center, [0.0, 1.0, 1.0], [0.0, 0.0]),
                    (front_right, [0.0, 1.0, 1.0], [0.0, 0.0]),
                    // Triangle on the right side connecting to the top middle
                    (front_right, [1.0, 1.0, 0.0], [0.0, 0.0]),
                    (top_center, [1.0, 1.0, 0.0], [0.0, 0.0]),
                    (back_right, [1.0, 1.0, 0.0], [0.0, 0.0]),
                    // Triangle on the back side connecting to the top middle
                    (back_right, [0.0, 1.0, -1.0], [0.0, 0.0]),
                    (top_center, [0.0, 1.0, -1.0], [0.0, 0.0]),
                    (back_left, [0.0, 1.0, -1.0], [0.0, 0.0]),
                ];
                let indices = Indices::U32(vec![
                    0, 1, 2, 2, 3, 0, // Base square
                    4, 5, 6, // Left side
                    7, 8, 9, // Front side
                    10, 11, 12, // Right side
                    13, 14, 15, // Back side
                ]);
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut uvs = Vec::new();

                for (position, normal, uv) in vertices.iter() {
                    positions.push(*position);
                    normals.push(*normal);
                    uvs.push(*uv);
                }
                let mut water_mesh = Mesh::new(PrimitiveTopology::TriangleList);

                water_mesh.set_indices(Some(indices));
                water_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                water_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                water_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(water_mesh),
                    // render_pipelines: RenderPipelines::from_pipelines(vec![
                    //     RenderPipeline::new(pipeline),
                    // ]),
                    material: materials.add(StandardMaterial {
                        base_color: Color::BLUE,
                        metallic: 0.2,
                        reflectance: 0.7,
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        });

    println!("Build completed");

    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    render_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![
        0.0;
        surface_nets_buffer
            .mesh
            .positions
            .len()
    ]);
    render_mesh.set_attribute("Vertex_Layer", vec![
        0.0;
        surface_nets_buffer
            .mesh
            .positions
            .len()
    ]);
    render_mesh.set_attribute(
        Mesh::ATTRIBUTE_POSITION,
        surface_nets_buffer.mesh.positions,
    );
    render_mesh.set_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        surface_nets_buffer.mesh.normals,
    );

    render_mesh
        .set_indices(Some(Indices::U32(surface_nets_buffer.mesh.indices)));

    // let pipeline =
    //     pipelines.add(PipelineDescriptor::default_config(ShaderStages {
    //         vertex:   shaders
    //             .add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
    //         fragment: Some(shaders.add(Shader::from_glsl(
    //             ShaderStage::Fragment,
    //             FRAGMENT_SHADER,
    //         ))),
    //     }));
    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(render_mesh),
    //     // render_pipelines: RenderPipelines::from_pipelines(vec![
    //     //     RenderPipeline::new(pipeline),
    //     // ]),
    //     material: materials.add(StandardMaterial {
    //         base_color: Color::WHITE,
    //         metallic: 1.0,
    //         reflectance: 0.3,
    //         ..Default::default()
    //     }),
    //     ..Default::default()
    // });
}
#[derive(Debug)]
struct NeighborHeights {
    p:  i32,
    n:  i32,
    nw: i32,
    w:  i32,
    sw: i32,
    s:  i32,
    se: i32,
    e:  i32,
    ne: i32,
}
impl NeighborHeights {
    const NORTH: PointN<[i32; 3]> = PointN([1, 0, 0]);
    const NORTH_WEST: PointN<[i32; 3]> = PointN([1, 0, -1]);
    const WEST: PointN<[i32; 3]> = PointN([0, 0, -1]);
    const SOUTH_WEST: PointN<[i32; 3]> = PointN([-1, 0, -1]);
    const SOUTH: PointN<[i32; 3]> = PointN([-1, 0, 0]);
    const SOUTH_EAST: PointN<[i32; 3]> = PointN([-1, 0, 1]);
    const EAST: PointN<[i32; 3]> = PointN([0, 0, 1]);
    const NORTH_EAST: PointN<[i32; 3]> = PointN([1, 0, 1]);

    pub fn new(
        extent: &Extent3i,
        world_array: &Array3x1<WorldVoxel>,
        p: PointN<[i32; 3]>,
    ) -> Self {
        trace!("Getting neighbor heights for {:?}", p);
        let global_neighbor_heights = Self {
            p:  Self::get_height(extent, world_array, p),
            n:  Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::NORTH,
            ),
            nw: Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::NORTH_WEST,
            ),
            w:  Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::WEST,
            ),
            sw: Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::SOUTH_WEST,
            ),
            s:  Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::SOUTH,
            ),
            se: Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::SOUTH_EAST,
            ),
            e:  Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::EAST,
            ),
            ne: Self::get_height(
                extent,
                world_array,
                p + NeighborHeights::NORTH_EAST,
            ),
        };
        let local_adjustment = global_neighbor_heights.min();
        NeighborHeights {
            p:  global_neighbor_heights.p - local_adjustment,
            n:  global_neighbor_heights.n - local_adjustment,
            nw: global_neighbor_heights.nw - local_adjustment,
            w:  global_neighbor_heights.w - local_adjustment,
            sw: global_neighbor_heights.sw - local_adjustment,
            s:  global_neighbor_heights.s - local_adjustment,
            se: global_neighbor_heights.se - local_adjustment,
            e:  global_neighbor_heights.e - local_adjustment,
            ne: global_neighbor_heights.ne - local_adjustment,
        }
    }
    fn avg(&self) -> f32 {
        // Averages the heights of the neighbors
        (self.p
            + self.n
            + self.s
            + self.w
            + self.e
            + self.nw
            + self.ne
            + self.sw
            + self.se) as f32
            / 9.0
    }
    fn min(&self) -> i32 {
        let mut min = self.p;
        if self.n < min {
            min = self.n;
        }
        if self.nw < min {
            min = self.nw;
        }
        if self.w < min {
            min = self.w;
        }
        if self.sw < min {
            min = self.sw;
        }
        if self.s < min {
            min = self.s;
        }
        if self.se < min {
            min = self.se;
        }
        if self.e < min {
            min = self.e;
        }
        if self.ne < min {
            min = self.ne;
        }
        min
    }

    fn sum(&self) -> i32 {
        self.p
            + self.n
            + self.nw
            + self.w
            + self.sw
            + self.s
            + self.se
            + self.e
            + self.ne
    }
    fn get_height(
        extent: &Extent3i,
        world_array: &Array3x1<WorldVoxel>,
        column: PointN<[i32; 3]>,
    ) -> i32 {
        // returns the highest non-empty y-value in the voxel's column
        let mut y = extent.shape.y();
        while y >= 0 {
            let p = PointN([column.x(), y, column.z()]);
            if world_array.contains(p) && !world_array.get(p).is_empty() {
                return y;
            }
            y -= 1;
        }
        0
    }
}

fn boolean_sdf(
    extent: Extent3i,
    world_array: &Array3x1<WorldVoxel>,
) -> Array3x1<f32> {
    Array3x1::fill_with(extent, |p| {
        if world_array.get(p).is_empty() {
            1.0
        } else {
            -1.0
        }
    })
}

fn averaged_sdf(
    extent: Extent3i,
    voxel_array: &Array3x1<f32>,
    radius: i32,
) -> Array3x1<f32> {
    // smooths an sdf by taking the mean of a (2r+1)x(2r+1)x(2r+1) cube
    Array3x1::fill_with(extent, |p| {
        let mut sum = 0.0;
        let mut count = 0;
        for x in -radius..=radius {
            for y in -radius..=radius {
                for z in -radius..=radius {
                    let p = p + PointN([x, y, z]);
                    if voxel_array.contains(p) {
                        sum += voxel_array.get(p);
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            0.0
        } else {
            sum / count as f32
        }
    })
}
// TODO: The rest of this file has been copy-pasted
const TEXTURE_LAYERS: u32 = 4;
const UV_SCALE: f32 = 0.1;
/// Default bevy vertex shader with added vertex attribute for texture layer
const VERTEX_SHADER: &str = r#"
#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;
layout(location = 3) in uint Vertex_Layer; // New thing

layout(location = 0) out vec3 v_Position;
layout(location = 1) out vec3 v_Normal;
layout(location = 2) out vec3 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    v_Normal = mat3(Model) * Vertex_Normal;
    v_Position = (Model * vec4(Vertex_Position, 1.0)).xyz;

    // Gets used here and passed to the fragment shader.
    v_Uv = vec3(Vertex_Uv, Vertex_Layer);

    gl_Position = ViewProj * vec4(v_Position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450

const int MAX_LIGHTS = 10;

struct Light {
    mat4 proj;
    vec4 pos;
    vec4 color;
};

layout(location = 0) in vec3 v_Position;
layout(location = 1) in vec3 v_Normal;
layout(location = 2) in vec3 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Lights {
    vec3 AmbientColor;
    uvec4 NumLights;
    Light SceneLights[MAX_LIGHTS];
};

layout(set = 3, binding = 0) uniform StandardMaterial_base_color {
    vec4 base_color;
};

layout(set = 3, binding = 1) uniform texture2DArray StandardMaterial_base_color_texture;
layout(set = 3, binding = 2) uniform sampler StandardMaterial_base_color_texture_sampler;

void main() {
    o_Target = base_color * texture(
        sampler2DArray(StandardMaterial_base_color_texture, StandardMaterial_base_color_texture_sampler),
        v_Uv
    );
}
"#;

/// Utility struct for building the mesh
#[derive(Debug, Default, Clone)]
struct MeshBuf {
    pub positions:  Vec<[f32; 3]>,
    pub normals:    Vec<[f32; 3]>,
    pub tex_coords: Vec<[f32; 2]>,
    pub layer:      Vec<u32>,
    pub indices:    Vec<u32>,
}

impl MeshBuf {
    fn add_quad(
        &mut self,
        face: &OrientedCubeFace,
        quad: &UnorientedQuad,
        u_flip_face: Axis3,
        layer: u32,
    ) {
        let voxel_size = 1.0;
        let start_index = self.positions.len() as u32;
        self.positions
            .extend_from_slice(&face.quad_mesh_positions(quad, voxel_size));
        self.normals.extend_from_slice(&face.quad_mesh_normals());

        let flip_v = true;
        let mut uvs = face.tex_coords(u_flip_face, flip_v, quad);
        for uv in uvs.iter_mut() {
            for c in uv.iter_mut() {
                *c *= UV_SCALE;
            }
        }
        self.tex_coords.extend_from_slice(&uvs);

        self.layer.extend_from_slice(&[layer; 4]);
        self.indices
            .extend_from_slice(&face.quad_mesh_indices(start_index));
    }
}
