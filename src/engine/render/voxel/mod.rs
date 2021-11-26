use bevy::{prelude::*,
           render::{mesh::Indices,
                    pipeline::{PipelineDescriptor, PrimitiveTopology,
                               RenderPipeline},
                    shader::{ShaderStage, ShaderStages},
                    texture::{AddressMode, SamplerDescriptor}}};
use building_blocks::mesh::{greedy_quads, GreedyQuadsBuffer, IsOpaque,
                            MergeVoxel, OrientedCubeFace, UnorientedQuad,
                            RIGHT_HANDED_Y_UP_CONFIG};
use building_blocks::prelude::*;
use vox_format::types::ColorIndex;

use crate::engine::asset::TextureAssets;

pub fn build(
    // Builds the world
    mut commands: Commands,
    texture_handle: Res<TextureAssets>,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    // Draw terrain
    let extent =
        Extent3i::from_min_and_shape(PointN([0; 3]), PointN([1000, 100, 1000]));
    let mut voxels = Array3x1::fill(extent, Voxel::default());
    let rock_level =
        Extent3i::from_min_and_shape(PointN([0, 0, 0]), PointN([10, 10, 10]));
    voxels.fill_extent(&rock_level, Voxel(1));

    // Load buildings TODO: Move these to asset_collections
    let barnhouse_array = building_blocks::storage::vox_format::from_file::<
        ColorIndex,
        _,
    >("assets/models/buildings/barnhouse.vox", 0)
    .expect("reading file failed")
    .expect("file not found");

    // let mut position = world::Position { x: 0, y: 0, z: 1 };
    let mut texture = textures.get_mut(&texture_handle.block_textures).unwrap();
    // TODO: The rest of this file has been copy-pasted
    texture.sampler = SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        ..Default::default()
    };

    texture.reinterpret_stacked_2d_as_array(TEXTURE_LAYERS);
    let mut greedy_buffer =
        GreedyQuadsBuffer::new(extent, RIGHT_HANDED_Y_UP_CONFIG.quad_groups());
    greedy_quads(&voxels, &extent, &mut greedy_buffer);

    let mut mesh_buf = MeshBuf::default();
    for group in greedy_buffer.quad_groups.iter() {
        for quad in group.quads.iter() {
            let mat = voxels.get(quad.minimum);
            mesh_buf.add_quad(
                &group.face,
                quad,
                RIGHT_HANDED_Y_UP_CONFIG.u_flip_face,
                mat.0 as u32 - 1,
            );
        }
    }

    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let MeshBuf {
        positions,
        normals,
        tex_coords,
        layer,
        indices,
    } = mesh_buf;

    render_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    render_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    render_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords);
    render_mesh.set_attribute("Vertex_Layer", layer);
    render_mesh.set_indices(Some(Indices::U32(indices)));

    let pipeline =
        pipelines.add(PipelineDescriptor::default_config(ShaderStages {
            vertex:   shaders
                .add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                FRAGMENT_SHADER,
            ))),
        }));
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(render_mesh),
        render_pipelines: RenderPipelines::from_pipelines(vec![
            RenderPipeline::new(pipeline),
        ]),
        material: materials.add(texture_handle.block_textures.clone().into()),
        ..Default::default()
    });
}

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

#[derive(Default, Clone, Copy)]
struct Voxel(u8);

impl MergeVoxel for Voxel {
    type VoxelValue = u8;

    fn voxel_merge_value(&self) -> Self::VoxelValue { self.0 }
}

impl IsOpaque for Voxel {
    fn is_opaque(&self) -> bool { true }
}

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool { self.0 == 0 }
}

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
// for asset in building_assets {
//         // Load .vox file
//         let building = dot_vox::load(asset).unwrap();
//         let vox_palette = &building.palette;
//         for voxel in &building.models[0].voxels {
//             let color_u32 = palette::rgb::Rgb::<
//                 palette::encoding::srgb::Srgb,
//                 u8,
//             >::from_u32::<palette::rgb::channels::Abgr>(
//                 vox_palette[voxel.i as usize],
//             );
//             let color = Color::rgb(
//                 color_u32.red as f32 / 255.0,
//                 color_u32.green as f32 / 255.0,
//                 color_u32.blue as f32 / 255.0,
//             );
//             commands.spawn().insert(Voxel {
//                 x:        (voxel.x as u32) + position.x as u32,
//                 y:        voxel.z as u32,
//                 z:        voxel.y as u32,
//                 material: color,
//             });
//         }
//         position.x += building.models[0].size.x;
//     }
//     for asset in object_assets {
//         // Load .vox file
//         let object = dot_vox::load(asset).unwrap();
//         let vox_palette = &object.palette;
//         for voxel in &object.models[0].voxels {
//             let color_u32 = palette::rgb::Rgb::<
//                 palette::encoding::srgb::Srgb,
//                 u8,
//             >::from_u32::<palette::rgb::channels::Abgr>(
//                 vox_palette[voxel.i as usize],
//             );
//             let color = Color::rgb(
//                 color_u32.red as f32 / 255.0,
//                 color_u32.green as f32 / 255.0,
//                 color_u32.blue as f32 / 255.0,
//             );
//             commands.spawn().insert(ObjectVoxel {
//                 x:        0.0,
//                 y:        0.0,
//                 z:        0.0,
//                 material: color,
//             });
//             voxel_count += 1;
//         }
//         position.x += object.models[0].size.x.saturating_div(10);
//     }
//     for asset in character_assets {
//         // Load .vox file
//         let character = dot_vox::load(asset).unwrap();
//         let vox_palette = &character.palette;
//         for voxel in &character.models[0].voxels {
//             let color_u32 = palette::rgb::Rgb::<
//                 palette::encoding::srgb::Srgb,
//                 u8,
//             >::from_u32::<palette::rgb::channels::Abgr>(
//                 vox_palette[voxel.i as usize],
//             );
//             let color = Color::rgb(
//                 color_u32.red as f32 / 255.0,
//                 color_u32.green as f32 / 255.0,
//                 color_u32.blue as f32 / 255.0,
//             );
//             commands.spawn().insert(ObjectVoxel {
//                 x:        0.0,
//                 y:        0.0,
//                 z:        0.0,
//                 material: color,
//             });
//             voxel_count += 1;
//         }
//         position.x += character.models[0].size.x.saturating_div(10);
//     }
//}
