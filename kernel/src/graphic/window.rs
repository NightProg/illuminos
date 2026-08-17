// use alloc::borrow::ToOwned;
// use alloc::string::{String, ToString};
// use alloc::vec::Vec;
// use crate::geo::{line, rect, rect_nofill};
// use crate::graphic::Color;
// use crate::graphic::font::Psf2Font;
// use crate::math::{vec2, Vec2};
// use crate::println;
// xx
// pub struct Window<'a, T: FrameBuffer> {
//     pub framebuffer: T,
//     pub width: usize,
//     pub height: usize,
//     pub v: Vec2<usize>,
//     pub title: String,
//     pub font: Psf2Font<'a>
// }

// impl<'a, T: FrameBuffer> Window<'a, T> {
//     pub fn new(framebuffer: T, width: usize, height: usize, title: &str, font: Psf2Font<'a>) -> Self {
//         let v = Vec2::new(0, 0);
//         Self {
//             framebuffer,
//             width,
//             height,
//             v,
//             title: title.to_string(),
//             font,
//         }
//     }

//     pub fn draw_border(&mut self) {
//         let real_width = self.width + 30;
//         let real_height = self.height + 30;
//         let win_grass = 10;

//         let win_top_left = Vec2::new(self.v.x, self.v.y);
//         let win_bottom_right = Vec2::new(self.v.x + real_width, self.v.y + real_height);

//         for i in 0..win_grass {
//             let top_left = Vec2::new(self.v.x + i, self.v.y + i);
//             let bottom_right = Vec2::new(self.v.x + real_width - i, self.v.y + real_height - i);
//             let rect = rect_nofill(top_left, bottom_right);
//             self.framebuffer.draw_object(&rect, Color::grey());
//         }

//         let top_left = Vec2::new(self.v.x + win_grass, self.v.y + win_grass);
//         let bottom_right = Vec2::new(self.v.x + real_width - win_grass, self.v.y + real_height - win_grass);

//         let mut top_center_x = ((top_left.x + bottom_right.x) / 2) as isize;
//         top_center_x -= (self.title.len() / 2 * self.font.width) as isize;
//         let str = if self.title.len() * self.font.width > (bottom_right.x - top_left.x) as usize {
//             top_center_x = top_left.x as isize;
//             let mut str = self.title.clone();
//             let new_len = (bottom_right.x - top_left.x) as usize / self.font.width;
//             str.truncate(new_len - 3);
//             str += "...";
//             str
//         } else {
//             self.title.clone()
//         };

//         self.framebuffer.draw_psf_string(vec2(top_center_x as usize,  top_left.y), &str, Color::white(), &self.font);

//         let top_left = Vec2::new(self.v.x + win_grass, self.v.y + win_grass + self.font.height);
//         let top_rigth = Vec2::new(self.v.x + real_width - win_grass, self.v.y + win_grass + self.font.height);
//         let line = line(top_left, top_rigth);
//         self.framebuffer.draw_object(&line, Color::white());
//         println!("{:?}", real_height);
//         println!("{:?}", real_width);
//         println!("{:?}", line);
//     }

// }
