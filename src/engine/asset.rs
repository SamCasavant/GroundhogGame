// This file contains all of the asset paths for the game

use bevy::prelude::*;
use bevy_asset_loader::{AssetCollection, AssetLoader};

use crate::engine;
pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(
        &self,
        mut app: &mut AppBuilder,
    ) {
        AssetLoader::new(
            engine::AppState::LoadingAssets,
            engine::AppState::BuildingWorld,
        )
        .with_collection::<TextureAssets>()
        .build(&mut app);
    }
}

#[derive(AssetCollection)]
pub struct TextureAssets {
    #[asset(path = "materials.png")]
    pub block_textures: Handle<Texture>,
}
