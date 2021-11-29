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
    let extent =
        Extent3i::from_min_and_shape(PointN([0; 3]), PointN([1000, 100, 1000]));
    let mut world_array = Array3x1::fill(extent, WorldVoxel::EMPTY);
    // Draw terrain
    let rock_level =
        Extent3i::from_min_and_shape(PointN([0, 0, 0]), PointN([100, 5, 100]));
    world_array.fill_extent(&rock_level, WorldVoxel(1));

    // Add buildings
    let barn_house_model = models
        .get(&building_handles.barn_house)
        .expect("barnhouse.vox failed to initialize");
    let barn_house_content = barn_house_model
        .voxels
        .borrow_channels(|voxel: Channel<WorldVoxel, &[WorldVoxel]>| voxel);
    let barn_house_size = barn_house_model.size;
    let barn_house_extent =
        Extent3i::from_min_and_shape(PointN([0, 0, 0]), barn_house_size);

    copy_extent(&barn_house_extent, &barn_house_content, &mut world_array);
    let mut surface_nets_buffer =
        building_blocks::mesh::surface_nets::SurfaceNetsBuffer::default();

    let mut world_sdf = Array3x1::fill_with(extent, |p| {
        if world_array.get(p).is_empty() {
            1.0
        } else {
            -1.0
        }
    });

    building_blocks::mesh::surface_nets::surface_nets(
        &mut world_sdf,
        &extent,
        // &building_blocks::mesh::surface_nets::
        // padded_surface_nets_chunk_extent(     &extent,
        // ),
        1.0,
        &mut surface_nets_buffer,
    );

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
