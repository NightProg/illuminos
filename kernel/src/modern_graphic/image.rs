use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use crate::modern_graphic::surface::Surface;
use super::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImageFormat {
    RGB32,
    RGBA32,
    BGR32,
    BGRA32,
}

impl ImageFormat {
    pub fn is_rgb(&self) -> bool {
        matches!(self, ImageFormat::RGB32 | ImageFormat::RGBA32)
    }
    pub fn is_bgr(&self) -> bool {
        matches!(self, ImageFormat::BGR32 | ImageFormat::BGRA32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageLayout {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub format: ImageFormat,
}

impl ImageLayout {
    pub fn new(format: ImageFormat, width: u32, height: u32) -> Self {
        let bpp = match format {
            ImageFormat::RGB32 | ImageFormat::BGR32 => 3,
            ImageFormat::RGBA32 | ImageFormat::BGRA32 => 4,
        };
        ImageLayout {
            width,
            height,
            bpp,
            format,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub layout: ImageLayout,
    pub data: Vec<u8>,
}

impl Image {

    pub fn load_from_png(data: &[u8]) -> Result<Image, String> {
        let png_signature = [137, 80, 78, 71, 13, 10, 26, 10];
        if data.len() < png_signature.len() {
            return Err("Data is too short to be a PNG".to_string());
        }
        if &data[0..png_signature.len()] != &png_signature {
            return Err("Data is not a valid PNG".to_string());
        }

        todo!("Implement PNG loading");

    }
    pub fn new(image_layout: ImageLayout) -> Self {
        Image {
            layout: image_layout,
            data: vec![0; (image_layout.width * image_layout.height * image_layout.bpp) as usize],
        }
    }

    pub fn write_pixel(&mut self, x: u32, y: u32, color: Color) {
        let index = ((y * self.layout.width + x) * self.layout.bpp) as usize;
        if self.layout.format.is_rgb() {
            for i in 0..self.layout.bpp {
                self.data[index + i as usize] = color[i as usize];
            }
        } else if self.layout.format.is_bgr() {
           self.data[index..index+self.layout.bpp as usize].copy_from_slice(&color.to_bgr_bytes());
        }
    }

    pub fn read_pixel(&self, x: u32, y: u32) -> &[u8] {
        let index = ((y * self.layout.width + x) * self.layout.bpp) as usize;
        &self.data[index..index + self.layout.bpp as usize]
    }

    pub fn to_format(&self, format: ImageFormat) -> Image {
        let mut new_image = Image::new(self.layout);
        new_image.layout.format = format;
        for y in 0..self.layout.height {
            for x in 0..self.layout.width {
                let pixel = self.read_pixel(x, y);
                new_image.write_pixel(x, y, Color::from_format_bytes(self.layout.format, pixel));
            }
        }
        new_image
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageRef {
    id: u64,
    layout: ImageLayout
}

impl ImageRef {
    pub fn new(id: u64, layout: ImageLayout) -> Self {
        ImageRef {
            id,
            layout
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn layout(&self) -> ImageLayout {
        self.layout
    }
}


#[derive(Debug, Clone)]
pub enum ImageState {
    Unset,
    Set(Image),
}

#[derive(Debug, Clone)]
pub struct ImageTable {
    images: BTreeMap<ImageRef, ImageState>,
    next_id: u64,
}

impl ImageTable {
    pub fn new() -> Self {
        ImageTable {
            images: BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn create_unset_image_ref(&mut self, layout: ImageLayout) -> ImageRef {
        let image_ref = ImageRef::new(self.next_id, layout);
        self.images.insert(image_ref, ImageState::Unset);
        self.next_id += 1;
        image_ref
    }

    pub fn set_image(&mut self, image_ref: ImageRef, image: Image) {
        if let Some(state) = self.images.get_mut(&image_ref) {
            *state = ImageState::Set(image);
        } else {
            self.images.insert(image_ref, ImageState::Set(image));
        }
    }

    pub fn get_image(&self, image_ref: ImageRef) -> Option<&Image> {
        if let Some(state) = self.images.get(&image_ref) {
            if let ImageState::Set(image) = state {
                return Some(image);
            }
        }
        None
    }

    pub fn get_mut_image(&mut self, image_ref: ImageRef) -> Option<&mut Image> {
        if let Some(state) = self.images.get_mut(&image_ref) {
            if let ImageState::Set(image) = state {
                return Some(image);
            }
        }
        None
    }

    pub fn get_mut_or_create_image(&mut self, image_ref: ImageRef) -> &mut Image {
        if let Some(state) = self.images.get_mut(&image_ref) {
            if let ImageState::Set(image) = state {
                return image;
            } else {
                let new_image = Image::new(image_ref.layout);
                *state = ImageState::Set(new_image);
                if let ImageState::Set(image) = state {
                    return image;
                }

            }
        }

        panic!("Failed to create or get image");
    }

    pub fn get_or_create_image(&mut self, image_ref: ImageRef) -> Image {
        if let Some(state) = self.images.get(&image_ref).cloned() {
            if let ImageState::Set(image) = state {
                return image;
            } else {
                let new_image = Image::new(image_ref.layout);
                self.images.insert(image_ref, ImageState::Set(new_image.clone()));
                return new_image;
            }
        }

        panic!("Failed to create or get image");
    }

    pub fn clear(&mut self) {
        self.images.clear();
        self.next_id = 0;
    }
}

#[derive(Clone)]
pub struct Swapchain {
    images: Vec<Image>,
    image_count: usize,
    current_image_index: usize,
    surface: Surface
}

impl Swapchain {
    pub fn new(surface: Surface, format: ImageFormat, image_count: usize) -> Self {
        let images = (0..image_count)
            .map(|_| Image::new(
                ImageLayout::new(format, surface.screen.info().width as u32, surface.screen.info().height as u32)
            ))
            .collect();
        Swapchain {
            surface,
            images,
            image_count,
            current_image_index: 0,
        }
    }

    pub fn current_image(&self) -> &Image {
        &self.images[self.current_image_index]
    }

    pub fn next_image(&mut self) {
        self.current_image_index = (self.current_image_index + 1) % self.image_count;
    }

    pub fn present(&mut self, image: &Image) {
        self.surface.present(image);
    }
}
