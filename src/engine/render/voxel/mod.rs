use bevy::{asset::LoadState,
           prelude::*,
           render::{mesh::Indices,
                    pipeline::{PipelineDescriptor, PrimitiveTopology,
                               RenderPipeline},
                    shader::{ShaderStage, ShaderStages},
                    texture::{AddressMode, SamplerDescriptor}}};
use building_blocks::mesh::{greedy_quads, GreedyQuadsBuffer, IsOpaque,
                            MergeVoxel, OrientedCubeFace, UnorientedQuad,
                            RIGHT_HANDED_Y_UP_CONFIG};
use building_blocks::prelude::*;

use crate::engine::world;

pub struct VoxelPlugin;
impl Plugin for VoxelPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        // todo: Copy-paste
        app.insert_resource(State::new(AppState::Loading))
            .add_state(AppState::Loading)
            .add_system_set(
                SystemSet::on_enter(AppState::Loading)
                    .with_system(load_textures.system()),
            )
            .add_system_set(
                SystemSet::on_update(AppState::Loading)
                    .with_system(check_loaded.system()),
            )
            .add_system_set(
                SystemSet::on_enter(AppState::Run)
                    .with_system(load_assets.system()),
            );
    }
}

pub fn load_assets(
    mut commands: Commands,
    texture_handle: Res<LoadingTexture>,
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

    let building_assets = [
        "assets/models/buildings/barnhouse.vox",
        "assets/models/buildings/windybean.vox",
    ];
    let object_assets = ["assets/models/objects/pot.vox"];
    let character_assets = ["assets/models/characters/temp.vox"];
    let mut position = world::Position { x: 0, y: 0, z: 1 };

    // TODO: The rest of this file has been copy-pasted
    let mut texture = textures.get_mut(&texture_handle.0).unwrap();
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
        material: materials.add(texture_handle.0.clone().into()),
        ..Default::default()
    });
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AppState {
    Loading,
    Run,
}
pub struct LoadingTexture(Handle<Texture>);

fn load_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let handle = asset_server.load("materials.png");
    commands.insert_resource(LoadingTexture(handle));
}

/// Make sure that our texture is loaded so we can change some settings on it
/// later
fn check_loaded(
    mut state: ResMut<State<AppState>>,
    handle: Res<LoadingTexture>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_load_state(&handle.0) {
        state.set(AppState::Run).unwrap();
    }
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
