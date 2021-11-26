use bevy::prelude::*;
use bevy_asset_loader::AssetCollection;

#[derive(AssetCollection)]
pub struct TextureAssets {
    #[asset(path = "materials.png")]
    pub block_textures: Handle<Texture>,
}
