pub use yuv::YuvError;
use yuv::{
    BufferStoreMut, YuvChromaSubsampling, YuvConversionMode, YuvPlanarImage, YuvPlanarImageMut,
    YuvRange, YuvStandardMatrix,
};

pub struct Rgb8Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct Yuv420Image {
    pub width: u32,
    pub height: u32,
    pub y_stride: u32,
    pub u_stride: u32,
    pub v_stride: u32,
    pub y_plane: Vec<u8>,
    pub u_plane: Vec<u8>,
    pub v_plane: Vec<u8>,
}

impl Rgb8Image {
    pub fn to_yuv(self) -> Result<Yuv420Image, YuvError> {
        let mut yuv =
            YuvPlanarImageMut::alloc(self.width, self.height, YuvChromaSubsampling::Yuv420);
        yuv::rgb_to_yuv420(
            &mut yuv,
            &self.data,
            self.width * 3,
            YuvRange::Full,
            YuvStandardMatrix::Bt709,
            YuvConversionMode::Balanced,
        )?;
        let (
            BufferStoreMut::Owned(y_plane),
            BufferStoreMut::Owned(u_plane),
            BufferStoreMut::Owned(v_plane),
        ) = (yuv.y_plane, yuv.u_plane, yuv.v_plane)
        else {
            unreachable!();
        };
        Ok(Yuv420Image {
            width: yuv.width,
            height: yuv.height,
            y_stride: yuv.y_stride,
            u_stride: yuv.u_stride,
            v_stride: yuv.v_stride,
            y_plane: y_plane,
            u_plane: u_plane,
            v_plane: v_plane,
        })
    }
}

impl Yuv420Image {
    pub fn to_rgb8(&self) -> Result<Rgb8Image, YuvError> {
        let Self {
            width,
            height,
            y_stride,
            u_stride,
            v_stride,
            y_plane,
            u_plane,
            v_plane,
        } = self;
        let planar = YuvPlanarImage {
            width: *width,
            height: *height,
            y_stride: *y_stride,
            u_stride: *u_stride,
            v_stride: *v_stride,
            y_plane,
            u_plane,
            v_plane,
        };
        let mut rgb_bytes = vec![0u8; (*width * *height * 3) as _];
        yuv::yuv420_to_rgb(
            &planar,
            &mut rgb_bytes,
            *width * 3,
            YuvRange::Full,
            YuvStandardMatrix::Bt709,
        )?;
        Ok(Rgb8Image {
            width: *width,
            height: *height,
            data: rgb_bytes,
        })
    }
}
