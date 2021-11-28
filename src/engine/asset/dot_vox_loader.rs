// TODO: Reread this when you better understand what is going on
use anyhow;
use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::reflect::TypeUuid;
use bevy::utils::BoxedFuture;
use building_blocks::mesh::{IsOpaque, MergeVoxel};
use building_blocks::prelude::{Array3x1, Extent3i, GetMut, IsEmpty, PointN};
use dot_vox::{DotVoxData, Model, Size, Voxel};

#[derive(TypeUuid)]
#[uuid = "a4d4aa0a-f18d-4c3b-939c-56aee51e4dea"]
pub struct VoxModel {
    pub voxels: Array3x1<WorldVoxel>,
    pub size:   PointN<[i32; 3]>,
}

impl VoxModel {
    pub fn decode_vox(vox_data: &DotVoxData) -> Self {
        let Model {
            size: Size { x, y, z },
            voxels,
        } = &vox_data.models[0];
        let shape = PointN([*x as i32, *z as i32, *y as i32]);
        let extent = Extent3i::from_min_and_shape(PointN([0, 0, 0]), shape);
        let mut map = Array3x1::fill(extent, WorldVoxel::EMPTY);
        for Voxel { x, y, z, i } in voxels.iter() {
            let point = PointN([*x as i32, *z as i32, *y as i32]);
            *map.get_mut(point) = WorldVoxel(*i + 1);
        }

        Self {
            voxels: map,
            size:   shape,
        }
    }
}

#[derive(Default)]
pub struct VoxModelLoader;
impl AssetLoader for VoxModelLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let model =
                VoxModel::decode_vox(&dot_vox::load_bytes(bytes).unwrap());

            load_context.set_default_asset(LoadedAsset::new(model));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["vox"] }
}

#[derive(Default, Clone, Copy)]
pub struct WorldVoxel(pub u8);

impl WorldVoxel {
    pub const EMPTY: Self = Self(0);
    pub const WATER: Self = Self(1);
    pub const STONE: Self = Self(2);
    pub const CONCRETE: Self = Self(3);
}

impl MergeVoxel for WorldVoxel {
    type VoxelValue = u8;

    fn voxel_merge_value(&self) -> Self::VoxelValue { self.0 }
}

impl IsOpaque for WorldVoxel {
    fn is_opaque(&self) -> bool { true }
}

impl IsEmpty for WorldVoxel {
    fn is_empty(&self) -> bool { self.0 == 0 }
}
