use std::ffi::c_void;

use super::GraphicsBackend;
use log::warn;
use openvr as vr;
use openxr as xr;
use windows::core::Interface;
use windows::Win32::Graphics::{
    Direct3D11::{ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_TEXTURE2D_DESC},
    Dxgi,
};

pub struct D3D11Data {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    images: Vec<ID3D11Texture2D>,
    format: u32,
}

fn texture_extent_from_bounds(
    desc: &D3D11_TEXTURE2D_DESC,
    bounds: vr::VRTextureBounds_t,
) -> xr::Rect2Di {
    let width_min = bounds.uMin * desc.Width as f32;
    let width_max = bounds.uMax * desc.Width as f32;
    let height_min = bounds.vMin * desc.Height as f32;
    let height_max = bounds.vMax * desc.Height as f32;

    xr::Rect2Di {
        extent: xr::Extent2Di {
            width: (width_max - width_min).abs() as i32,
            height: (height_max - height_min).abs() as i32,
        },
        offset: xr::Offset2Di {
            x: width_min.min(width_max) as i32,
            y: height_min.min(height_max) as i32,
        },
    }
}

impl GraphicsBackend for D3D11Data {
    type Api = xr::D3D11;
    type OpenVrTexture = *mut c_void;
    type NiceFormat = Dxgi::Common::DXGI_FORMAT;

    #[inline]
    fn to_nice_format(format: <Self::Api as openxr::Graphics>::Format) -> Self::NiceFormat {
        Dxgi::Common::DXGI_FORMAT(format.try_into().unwrap())
    }

    fn session_create_info(&self) -> <Self::Api as openxr::Graphics>::SessionCreateInfo {
        xr::d3d::SessionCreateInfoD3D11 {
            device: self.device.as_raw(),
        }
    }

    fn get_texture(texture: &vr::Texture_t) -> Option<Self::OpenVrTexture> {
        if !texture.handle.is_null() {
            Some(texture.handle)
        } else {
            None
        }
    }

    fn store_swapchain_images(&mut self, images: Vec<Self::OpenVrTexture>, format: u32) {
        let images: Vec<_> = images
            .into_iter()
            .map(|p| unsafe { ID3D11Texture2D::from_raw(p) })
            .collect();

        self.images = images;
        self.format = format;
    }

    fn swapchain_info_for_texture(
        &self,
        texture: Self::OpenVrTexture,
        bounds: vr::VRTextureBounds_t,
        color_space: vr::EColorSpace,
    ) -> xr::SwapchainCreateInfo<Self::Api> {
        let texture = unsafe { ID3D11Texture2D::from_raw(texture) };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe {
            texture.GetDesc(&mut desc);
        }

        let xr::Rect2Di { extent, .. } = texture_extent_from_bounds(&desc, bounds);
        xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                | xr::SwapchainUsageFlags::TRANSFER_DST,
            format: desc.Format.0 as u32,
            sample_count: desc.SampleDesc.Count,
            width: extent.width as u32,
            height: extent.height as u32,
            face_count: 1,
            array_size: 2,
            mip_count: 1,
        }
    }

    fn copy_texture_to_swapchain(
        &self,
        _eye: vr::EVREye,
        _texture: Self::OpenVrTexture,
        _color_space: vr::EColorSpace,
        _bounds: vr::VRTextureBounds_t,
        _image_index: usize,
        _submit_flags: vr::EVRSubmitFlags,
    ) -> xr::Extent2Di {
        todo!()
    }

    fn copy_overlay_to_swapchain(
        &mut self,
        _texture: Self::OpenVrTexture,
        _bounds: vr::VRTextureBounds_t,
        _image_index: usize,
    ) -> xr::Extent2Di {
        todo!()
    }
}
impl D3D11Data {
    pub fn new(texture: ID3D11Texture2D) -> Option<Self> {
        let device = unsafe { texture.GetDevice() }
            .inspect_err(|e| log::error!("Failed to get D3D11 device: {e}"))
            .ok()?;
        let context = unsafe { device.GetImmediateContext() }
            .inspect_err(|e| log::error!("Failed to get D3D11 device context: {e}"))
            .ok()?;

        Some(D3D11Data {
            device,
            context,
            images: Default::default(),
            format: 0,
        })
    }
}
