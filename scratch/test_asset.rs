use bevy::prelude::*;
use bevy::render::render_asset::{RenderAsset, RenderAssetUsages, PrepareAssetError};
use bevy::render::render_resource::{Buffer, BufferInitDescriptor, BufferUsages};
use bevy::render::renderer::RenderDevice;
use bevy::ecs::system::lifetimeless::SRes;
use bevy::ecs::system::SystemParamItem;

#[derive(Asset, TypePath, Clone)]
pub struct ShaderBuffer { pub data: Vec<u8> }
pub struct GpuShaderBuffer { pub buffer: Buffer }

impl RenderAsset for ShaderBuffer {
    type SourceAsset = ShaderBuffer;
    type Param = SRes<RenderDevice>;
    type PreparedAsset = GpuShaderBuffer;

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: bevy::asset::AssetId<Self::SourceAsset>,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::SourceAsset>> {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("ShaderBuffer"),
            contents: &source_asset.data,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        Ok(GpuShaderBuffer { buffer })
    }

    fn asset_usage(source_asset: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }
}
fn main() {}
