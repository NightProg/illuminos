use crate::modern_graphic::image::{Image, ImageTable};
use crate::modern_graphic::{Color, Vertex};
use crate::modern_graphic::command::Command;
use crate::modern_graphic::command::CommandBuffer;
use alloc::vec::Vec;
use alloc::vec;
use crate::math::Arithmetic;

static GLOBAL_QUEUE: Queue = Queue {
    polygon_fill: true,
};

fn edge(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (c.0 - a.0) * (b.1 - a.1) - (c.1 - a.1) * (b.0 - a.0)
}

fn barycentric(a: (f32, f32), b: (f32, f32), c: (f32, f32), p: (f32, f32)) -> (f32, f32, f32) {
    let v0 = (b.0 - a.0, b.1 - a.1);
    let v1 = (c.0 - a.0, c.1 - a.1);
    let v2 = (p.0 - a.0, p.1 - a.1);

    let d00 = v0.0 * v0.0 + v0.1 * v0.1;
    let d01 = v0.0 * v1.0 + v0.1 * v1.1;
    let d11 = v1.0 * v1.0 + v1.1 * v1.1;
    let d20 = v2.0 * v0.0 + v2.1 * v0.1;
    let d21 = v2.0 * v1.0 + v2.1 * v1.1;

    let denom = d00 * d11 - d01 * d01;
    if denom == 0.0 {
        return (-1.0, -1.0, -1.0); // dégénéré
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    (u, v, w)
}


#[derive(Debug, Clone, Copy)]
pub struct Queue {
    polygon_fill: bool,
}



impl Queue {
    pub fn new(polygon_fill: bool) -> Self {
        Self {
            polygon_fill,
        }
    }

    fn interpolate_triangle(tri: [&Vertex; 3]) -> Vec<(isize, isize, Color)> {
        let a = (tri[0].x, tri[0].y);
        let b = (tri[1].x, tri[1].y);
        let c = (tri[2].x, tri[2].y);

        let area = edge(a, b, c);
        if area.abs() < f32::EPSILON {
            return vec![];
        }

        let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as isize;
        let max_x = a.0.max(b.0).max(c.0).ceil() as isize;
        let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as isize;
        let max_y = a.1.max(b.1).max(c.1).ceil() as isize;

        let mut output = vec![];

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = (x as f32 + 0.5, y as f32 + 0.5);

                let w0 = edge(b, c, p) / area;
                let w1 = edge(c, a, p) / area;
                let w2 = edge(a, b, p) / area;

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let r = w0 * tri[0].color[0].as_f32() + w1 * tri[1].color[0].as_f32() + w2 * tri[2].color[0].as_f32();
                    let g = w0 * tri[0].color[1].as_f32() + w1 * tri[1].color[1].as_f32() + w2 * tri[2].color[1].as_f32();
                    let b = w0 * tri[0].color[2].as_f32() + w1 * tri[1].color[2].as_f32() + w2 * tri[2].color[2].as_f32();

                    output.push((x, y, Color::new(r as u8, g as u8, b as u8, 0xff)));
                }
            }
        }

        output
    }

    fn fill_polygon(vertices: Vec<(isize, isize, Color)>) -> Vec<(isize, isize, Color)> {
        let n = vertices.len();
        assert!(n >= 3, "At least 3 vertices required");

        let verts: Vec<Vertex> = vertices
            .into_iter()
            .map(|(x, y, color)| Vertex {
                x: x as f32,
                y: y as f32,
                color,
            })
            .collect();

        let mut result = vec![];

        for i in 1..(verts.len() - 1) {
            let tri = [&verts[0], &verts[i], &verts[i + 1]];
            result.extend(Self::interpolate_triangle(tri));
        }

        result
    }

    fn get_line(&self, v0: Vertex, v1: Vertex) -> Vec<(isize, isize, Color)> {
        let mut coords = Vec::new();
        let dx = (v1.x - v0.x).abs();
        let dy = (v1.y - v0.y).abs();
        let sx = if v0.x < v1.x { 1 } else { -1 };
        let sy = if v0.y < v1.y { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = v0.x as isize;
        let mut y = v0.y as isize;

        while x != v1.x as isize || y != v1.y as isize {
            coords.push((x, y, v0.color));
            let err2 = err * 2.0;
            if err2 > -dy {
                err -= dy;
                x += sx;
            }
            if err2 < dx {
                err += dx;
                y += sy;
            }
        }

        coords
    }

    fn draw(&self, vertex: Vec<Vertex>, image: &mut Image) {
        let mut coords = Vec::new();

        for i in 0..vertex.len() {
            let v0 = vertex[i];
            let v1 = vertex[(i + 1) % vertex.len()];
            let line = self.get_line(v0, v1);
            coords.extend(line);
        }

        if self.polygon_fill {
            if vertex.len() == 3 {
                let tri = [&vertex[0], &vertex[1], &vertex[2]];
                let filled_coords = Self::interpolate_triangle(tri);
                for (x, y, color) in filled_coords {
                    image.write_pixel(x as u32, y as u32, color);
                }
            } else if vertex.len() > 3 {
                if vertex.len() % 3 == 0 {
                    for i in (0..vertex.len()).step_by(3) {
                        let tri = [&vertex[i], &vertex[i + 1], &vertex[i + 2]];
                        let filled_coords = Self::interpolate_triangle(tri);
                        for (x, y, color) in filled_coords {
                            image.write_pixel(x as u32, y as u32, color);
                        }
                    }
                } else {
                    let filled_coords = Self::fill_polygon(coords);
                    for (x, y, color) in filled_coords {
                        image.write_pixel(x as u32, y as u32, color);
                    }
                }
            }
        } else {
            for (x, y, color) in coords {
                image.write_pixel(x as u32, y as u32, color);
            }
        }
    }

    fn rasterize_triangle(
        &self,
        v0: ([f32; 2], (f32, f32)),
        v1: ([f32; 2], (f32, f32)),
        v2: ([f32; 2], (f32, f32)),
        target: &mut Image,
        texture: &Image
    ) {
        let min_x = v0.0[0].min(v1.0[0]).min(v2.0[0]).floor() as i32;
        let max_x = v0.0[0].max(v1.0[0]).max(v2.0[0]).ceil() as i32;
        let min_y = v0.0[1].min(v1.0[1]).min(v2.0[1]).floor() as i32;
        let max_y = v0.0[1].max(v1.0[1]).max(v2.0[1]).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                let (w0, w1, w2) = barycentric(
                    (v0.0[0], v0.0[1]),
                    (v1.0[0], v1.0[1]),
                    (v2.0[0], v2.0[1]),
                    (px, py),
                );

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let u = w0 * v0.1.0 + w1 * v1.1.0 + w2 * v2.1.0;
                    let v = w0 * v0.1.1 + w1 * v1.1.1 + w2 * v2.1.1;

                    let tex_x = (u * texture.layout.width as f32).clamp(0.0, texture.layout.width as f32 - 1.0) as u32;
                    let tex_y = (v * texture.layout.height as f32).clamp(0.0, texture.layout.height as f32 - 1.0) as u32;
                    let color = texture.read_pixel(tex_x, tex_y);

                    target.write_pixel(x as u32, y as u32, Color::from_format_bytes(texture.layout.format, color));
                }
            }
        }
    }


    fn draw_texture(&self, vertex: Vec<Vertex>, image: &mut Image, texture: &Image, uv: Vec<(f32, f32)>) {
        if vertex.len() == 3 {
            let v0 = vertex[0];
            let uv0 = uv[0];
            let v1 = vertex[1];
            let uv1 = uv[1];
            let v2 = vertex[2];
            let uv2 = uv[2];
            self.rasterize_triangle(
                ([v0.x, v0.y], uv0),
                ([v1.x, v1.y], uv1),
                ([v2.x, v2.y], uv2),
                image,
                texture,
            );
        } else if vertex.len() % 3 == 0 {
            for i in (0..vertex.len()).step_by(3) {
                let v0 = vertex[i];
                let uv0 = uv[i];
                let v1 = vertex[i + 1];
                let uv1 = uv[i + 1];
                let v2 = vertex[i + 2];
                let uv2 = uv[i + 2];
                self.rasterize_triangle(
                    ([v0.x, v0.y], uv0),
                    ([v1.x, v1.y], uv1),
                    ([v2.x, v2.y], uv2),
                    image,
                    texture,
                );
            }
        } else {
            panic!("Invalid vertex count for texture drawing");
        }


    }

    fn copy_image(&self, src: &Image, dst: &mut Image) {
        if src.layout.width > dst.layout.width || src.layout.height > dst.layout.height {
            panic!("Source image is larger than destination image");
        }
        let mut src = if src.layout.format != dst.layout.format {
            src.to_format(dst.layout.format)
        } else {
            src.clone()
        };
        for y in 0..src.layout.height {
            for x in 0..src.layout.width {
                let color = src.read_pixel(x, y);
                dst.write_pixel(x, y, Color::from_format_bytes(src.layout.format, color));
            }
        }
    }


    pub fn submit(
        &self,
        command_buffer: CommandBuffer
    )  -> ImageTable {
        let mut image_table = command_buffer.image_table.clone();
        let mut vertex = Vec::new();
        let mut index = Vec::new();
        let mut uv = Vec::new();
        let mut texture = None;
        for command in command_buffer.commands {
            match command {
                Command::ClearImage { color, target } => {
                    let image = image_table.get_mut_or_create_image(target);
                    for y in 0..image.layout.height {
                        for x in 0..image.layout.width {
                            image.write_pixel(x, y, color);
                        }
                    }
                },
                Command::BindVertex { vertex: bind_vertex } => {
                    vertex = bind_vertex;
                },
                Command::BindIndex { index: bind_index } => {
                    index = bind_index;
                },
                Command::BindUV { uv: bind_uv } => {
                    uv = bind_uv;
                },
                Command::BindTexture { texture: bind_texture } => {
                    let bind_texture = image_table.get_image(bind_texture).unwrap();
                    texture = Some(bind_texture.clone());
                },
                Command::CopyImage { src, dst } => {
                    let src_image = image_table.get_image(src).cloned().unwrap();
                    let dst_image = image_table.get_mut_or_create_image(dst);
                    self.copy_image(&src_image, dst_image);
                },
                Command::CallSubCommandBuffer { sub_command_buffer, out } => {
                    let sub_image = self.submit(sub_command_buffer.clone());
                    let image = sub_image.get_image(sub_command_buffer.main_image_target.unwrap()).unwrap();
                    image_table.set_image(out, image.clone());
                },
                Command::Draw { count, dst } => {
                    let image = image_table.get_mut_or_create_image(dst);
                    self.draw(vertex.clone(), image);
                },
                Command::DrawIndexed { count, dst } => {
                    let image = image_table.get_mut_or_create_image(dst);
                    let mut indexed_vertex = Vec::new();
                    for i in 0..count {
                        if i < index.len() as u32 {
                            indexed_vertex.push(vertex[index[i as usize] as usize]);
                        }
                    }
                    self.draw(indexed_vertex, image);

                },
                Command::DrawTexture { count, dst } => {
                    let image = image_table.get_mut_or_create_image(dst);
                    self.draw_texture(vertex.clone(), image, texture
                        .as_ref()
                        .unwrap()
                                      , uv.clone());
                }
                Command::DrawTextureIndexed { count, dst } => {
                    let image = image_table.get_mut_or_create_image(dst);
                    let mut indexed_vertex = Vec::new();
                    let mut indexed_uv = Vec::new();
                    for i in 0..count {
                        if i < index.len() as u32 {
                            indexed_vertex.push(vertex[index[i as usize] as usize]);
                        }
                    }
                    for i in 0..count {
                        if i < index.len() as u32 {
                            indexed_uv.push(uv[index[i as usize] as usize]);
                        }
                    }
                    self.draw_texture(indexed_vertex, image, texture
                        .as_ref()
                        .unwrap()
                                      , indexed_uv.clone());
                },

            }
        }

        image_table

    }
}