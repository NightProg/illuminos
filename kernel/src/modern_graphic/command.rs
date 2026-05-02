use super::image::{Image, ImageFormat, ImageLayout, ImageRef, ImageTable};
use super::{Color, Vertex};
use alloc::vec::Vec;


#[derive(Debug, Clone)]
pub enum Command {
    ClearImage {
        color: Color,
        target: ImageRef,
    },
    BindVertex {
        vertex: Vec<Vertex>,
    },
    BindUV {
        uv: Vec<(f32, f32)>,
    },
    BindIndex {
        index: Vec<u32>,
    },
    BindTexture {
        texture: ImageRef,
    },
    CopyImage {
        src: ImageRef,
        dst: ImageRef
    },
    CallSubCommandBuffer {
        sub_command_buffer: CommandBuffer,
        out: ImageRef
    },
    Draw {
        count: u32,
        dst: ImageRef,
    },
    DrawIndexed {
        count: u32,
        dst: ImageRef,
    },
    DrawTexture {
        count: u32,
        dst: ImageRef,
    },
    DrawTextureIndexed {
        count: u32,
        dst: ImageRef,
    },
}

#[derive(Debug, Clone)]
pub struct CommandBuffer {
    pub commands: Vec<Command>,
    pub image_table: ImageTable,
    pub main_image_target: Option<ImageRef>,
}
impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            image_table: ImageTable::new(),
            main_image_target: None,
        }
    }

    pub fn set_main_image_target(
        &mut self,
        image: Image,
    ) -> &mut Self {
        let image_ref = self.image_table.create_unset_image_ref(image.layout);
        self.image_table.set_image(image_ref, image);
        self.main_image_target = Some(image_ref);
        self
    }


    pub fn add_image(
        &mut self,
        image: Image,
    ) -> ImageRef {
        let image_ref = self.image_table.create_unset_image_ref(image.layout);
        self.image_table.set_image(image_ref, image);
        image_ref
    }

    pub fn add_target(
        &mut self,
        layout: ImageLayout
    ) -> ImageRef {
        let image_ref = self.image_table.create_unset_image_ref(layout);
        image_ref
    }

    pub fn clear_image(
        &mut self,
        color: Color,
    ) -> Option<&mut Self> {

        self.commands.push(Command::ClearImage { color, target: self.main_image_target? });
        Some(self)
    }
    pub fn clear_image_on(
        &mut self,
        color: Color,
        target: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::ClearImage { color, target });
        self
    }

    pub fn bind_vertex(
        &mut self,
        vertex: Vec<Vertex>,
    ) -> &mut Self {
        self.commands.push(Command::BindVertex { vertex });
        self
    }

    pub fn bind_uv(
        &mut self,
        uv: Vec<(f32, f32)>,
    ) -> &mut Self {
        self.commands.push(Command::BindUV { uv });
        self
    }

    pub fn bind_index(
        &mut self,
        index: Vec<u32>,
    ) -> &mut Self {
        self.commands.push(Command::BindIndex { index });
        self
    }

    pub fn bind_texture(
        &mut self,
        texture: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::BindTexture { texture });
        self
    }

    pub fn copy_image(
        &mut self,
        src: ImageRef,
    ) -> Option<&mut Self> {
        self.commands.push(Command::CopyImage { src, dst: self.main_image_target? });
        Some(self)
    }

    pub fn copy_image_on(
        &mut self,
        src: ImageRef,
        dst: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::CopyImage { src, dst });
        self
    }

    pub fn call_sub_command_buffer(
        &mut self,
        sub_command_buffer: CommandBuffer,
    ) -> ImageRef {
        let image_ref = self.image_table.create_unset_image_ref(sub_command_buffer.main_image_target.unwrap().layout());
        self.commands.push(Command::CallSubCommandBuffer { sub_command_buffer, out: image_ref });
        image_ref
    }

    pub fn draw(
        &mut self,
        count: u32,
    ) -> Option<&mut Self> {
        self.commands.push(Command::Draw {
            count,
            dst: self.main_image_target?,
        });
        Some(self)
    }

    pub fn draw_on(
        &mut self,
        count: u32,
        target: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::Draw {
            count,
            dst: target,
        });
        self
    }

    pub fn draw_indexed(
        &mut self,
        count: u32,
    ) -> Option<&mut Self> {
        self.commands.push(Command::DrawIndexed {
            count,
            dst: self.main_image_target?,
        });

        Some(self)
    }

    pub fn draw_indexed_on(
        &mut self,
        count: u32,
        target: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::DrawIndexed {
            count,
            dst: target,
        });
        self
    }

    pub fn draw_texture(
        &mut self,
        count: u32,
    ) -> Option<&mut Self> {
        self.commands.push(Command::DrawTexture {
            count,
            dst: self.main_image_target?,
        });

        Some(self)
    }

    pub fn draw_texture_on(
        &mut self,
        count: u32,
        target: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::DrawTexture {
            count,
            dst: target,
        });
        self
    }

    pub fn draw_texture_indexed(
        &mut self,
        count: u32,
    ) -> Option<&mut Self> {
        self.commands.push(Command::DrawTextureIndexed {
            count,
            dst: self.main_image_target?,
        });

        Some(self)
    }

    pub fn draw_texture_indexed_on(
        &mut self,
        count: u32,
        target: ImageRef,
    ) -> &mut Self {
        self.commands.push(Command::DrawTextureIndexed {
            count,
            dst: target,
        });
        self
    }

    pub fn execute_with(
        &mut self,
        queue: &super::queue::Queue,
    ) -> Option<Image> {
        let mut image_table = queue.submit(self.clone());
        let image = image_table.get_image(self.main_image_target?).unwrap();

        Some(image.clone())
    }

    pub fn execute_with_on(
        &mut self,
        queue: &super::queue::Queue,
        target: ImageRef,
    ) -> Option<Image> {
        let mut image_table = queue.submit(self.clone());
        let image = image_table.get_image(target).unwrap();

        Some(image.clone())
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.image_table.clear();
        self.main_image_target = None;
    }

}