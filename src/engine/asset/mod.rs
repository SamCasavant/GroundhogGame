// This file contains all of the asset paths for the game and custom loaders for
// unique data types

pub mod dot_vox_loader;

use bevy::prelude::*;
use bevy_asset_loader::{AssetCollection, AssetLoader};

use crate::engine;
pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(
        &self,
        mut app: &mut AppBuilder,
    ) {
        app.add_asset::<dot_vox_loader::VoxModel>()
            .init_asset_loader::<dot_vox_loader::VoxModelLoader>();
        AssetLoader::new(
            engine::AppState::LoadingAssets,
            engine::AppState::BuildingWorld,
        )
        .with_collection::<TextureAssets>()
        .with_collection::<BuildingAssets>()
        .build(app);
    }
}

#[derive(AssetCollection)]
pub struct TextureAssets {
    #[asset(path = "materials.png")]
    pub block_textures: Handle<Texture>,
}

#[derive(AssetCollection)]
pub struct BuildingAssets {
    #[asset(path = "models/buildings/barnhouse.vox")]
    pub barn_house: Handle<dot_vox_loader::VoxModel>,
}
