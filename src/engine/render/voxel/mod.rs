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
    let extent =
        Extent3i::from_min_and_shape(PointN([0; 3]), PointN([200, 100, 200]));
    let mut world_array = Array3x1::fill(extent, WorldVoxel::EMPTY);
    // Draw terrain1
    let dirt_level =
        Extent3i::from_min_and_shape(PointN([0, 0, 0]), PointN([200, 2, 200]));
    world_array.fill_extent(&dirt_level, WorldVoxel(1));

    // Construct a pyramid
    let pyramid_position = PointN([0, 2, 0]);
    let pyramid_extent =
        Extent3i::from_min_and_shape(pyramid_position, PointN([80, 80, 80]));
    let mut pyramid_array = Array3x1::fill(pyramid_extent, WorldVoxel::EMPTY);
    for y in 0..20 {
        let side_length: i32 = 20 - y;
        let corner = pyramid_position + PointN([y; 3]);
        let layer = Extent3i::from_min_and_shape(
            corner,
            PointN([side_length * 2, 1, side_length * 2]),
        );
        pyramid_array.fill_extent(&layer, WorldVoxel(1));
    }
    copy_extent(&pyramid_extent, &pyramid_array, &mut world_array);

    let tall_pyramid_position = PointN([60, 2, 60]);
    let tall_pyramid_extent = Extent3i::from_min_and_shape(
        tall_pyramid_position,
        PointN([40, 80, 40]),
    );
    let mut tall_pyramid_array =
        Array3x1::fill(tall_pyramid_extent, WorldVoxel::EMPTY);
    for y in 0..20 {
        let side_length = 40 - y * 2;
        let corner = tall_pyramid_position + PointN([y, 2 * y, y]);
        let layer = Extent3i::from_min_and_shape(
            corner,
            PointN([side_length, 2 * y, side_length]),
        );
        tall_pyramid_array.fill_extent(&layer, WorldVoxel(1));
    }
    copy_extent(&tall_pyramid_extent, &tall_pyramid_array, &mut world_array);

    // Add buildings
    let barn_house = models
        .get(&building_handles.barn_house)
        .expect("barnhouse.vox failed to initialize");
    let barn_house_content = barn_house
        .voxels
        .borrow_channels(|voxel: Channel<WorldVoxel, &[WorldVoxel]>| voxel);
    // copy_extent(&barn_house.extent, &barn_house.content, &mut world_array);
    let mut surface_nets_buffer =
        building_blocks::mesh::surface_nets::SurfaceNetsBuffer::default();
    let world_sdf = boolean_sdf(extent, &world_array);
    let averaged_world_sdf =
        averaged_sdf(extent, &averaged_sdf(extent, &world_sdf, 1), 1);

    building_blocks::mesh::surface_nets::surface_nets(
        &averaged_world_sdf,
        &extent,
        2.0,
        &mut surface_nets_buffer,
    );
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
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(render_mesh),
        // render_pipelines: RenderPipelines::from_pipelines(vec![
        //     RenderPipeline::new(pipeline),
        // ]),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 1.0,
            reflectance: 0.3,
            ..Default::default()
        }),
        ..Default::default()
    });
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
        sum / count as f32
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
