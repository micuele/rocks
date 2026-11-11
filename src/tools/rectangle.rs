use anyhow::Result;
use femtovg::{FontId, Paint, Path};
use relm4::{Sender, gtk::gdk::Key};

use crate::{
    configuration::APP_CONFIG,
    math::Vec2D,
    sketch_board::{MouseButton, MouseEventMsg, MouseEventType, SketchBoardInput},
    style::Style,
};

use super::{
    Drawable, DrawableClone, Tool, ToolUpdateResult, Tools,
    drag_box::{DragBox, draw_center_marker},
};

#[derive(Clone, Debug)]
pub struct Rectangle {
    origin: Vec2D,
    top_left: Vec2D,
    size: Option<Vec2D>,
    style: Style,
    centered: bool,
    finishing: bool,
    comment: Option<String>,
    number: usize,
}

impl Drawable for Rectangle {
    fn region_number(&self) -> usize {
        self.number
    }
    fn comment_badge(&self, bounds: (Vec2D, Vec2D)) -> Option<Vec2D> {
        self.comment.as_ref()?;
        let size = self.size?;
        let left = self.top_left.x.min(self.top_left.x + size.x);
        let right = self.top_left.x.max(self.top_left.x + size.x);
        let top = self.top_left.y.min(self.top_left.y + size.y);
        let bottom = self.top_left.y.max(self.top_left.y + size.y);
        let gap = 18.0
            + self
                .style
                .size
                .to_line_width(self.style.annotation_size_factor)
                / 2.0;
        let x = (left + 18.0)
            .max(bounds.0.x + 16.0)
            .min((bounds.1.x - 16.0).max(bounds.0.x + 16.0));
        let y = (top + 18.0)
            .max(bounds.0.y + 16.0)
            .min((bounds.1.y - 16.0).max(bounds.0.y + 16.0));
        let candidates = [
            Vec2D::new(x, top - gap),
            Vec2D::new(left - gap, y),
            Vec2D::new(x, bottom + gap),
            Vec2D::new(right + gap, y),
        ];
        Some(
            candidates
                .into_iter()
                .find(|p| {
                    p.x - 16.0 >= bounds.0.x
                        && p.y - 16.0 >= bounds.0.y
                        && p.x + 16.0 <= bounds.1.x
                        && p.y + 16.0 <= bounds.1.y
                })
                .unwrap_or(candidates[0]),
        )
    }
    fn needs_comment(&self) -> bool {
        self.comment.is_none()
    }
    fn region_comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    fn set_region_comment(&mut self, number: usize, comment: String) {
        self.number = number;
        self.comment = Some(comment);
    }
    fn set_region_number(&mut self, number: usize) {
        self.number = number;
    }
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
        bounds: (Vec2D, Vec2D),
    ) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };

        canvas.save();
        let mut path = Path::new();
        path.rounded_rect(
            self.top_left.x,
            self.top_left.y,
            size.x,
            size.y,
            APP_CONFIG.read().corner_roundness(),
        );

        if !self.finishing && self.centered {
            draw_center_marker(canvas, self.origin);
        }

        if self.style.fill {
            canvas.fill_path(&path, &self.style.into());
        } else {
            canvas.stroke_path(&path, &self.style.into());
        }
        if self.comment.is_some() {
            let center = self.comment_badge(bounds).unwrap();
            let (x, y) = (center.x, center.y);
            let mut badge = Path::new();
            badge.circle(x, y, 16.0);
            canvas.fill_path(&badge, &Paint::color(self.style.color.into()));
            let mut text = Paint::color(self.style.color.contrast().into());
            text.set_font(&[font]);
            text.set_font_size(20.0);
            text.set_text_align(femtovg::Align::Center);
            text.set_text_baseline(femtovg::Baseline::Middle);
            canvas.fill_text(x, y, self.number.to_string(), &text)?;
        }
        canvas.restore();

        Ok(())
    }
}

impl Rectangle {
    fn calculate_shape(&mut self, event: &MouseEventMsg) {
        let drag_box = DragBox::from_origin_delta(self.origin, event.pos, event.modifier);
        self.centered = drag_box.centered;
        self.top_left = drag_box.top_left;
        self.size = Some(drag_box.size);
    }
}

#[derive(Default)]
pub struct RectangleTool {
    rectangle: Option<Rectangle>,
    style: Style,
    sender: Option<Sender<SketchBoardInput>>,
}

impl Tool for RectangleTool {
    fn active(&self) -> bool {
        self.rectangle.is_some()
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                if event.button == MouseButton::Middle {
                    return ToolUpdateResult::Unmodified;
                }
                // start new
                self.rectangle = Some(Rectangle {
                    origin: event.pos,
                    top_left: event.pos,
                    size: None,
                    style: self.style,
                    centered: false,
                    finishing: false,
                    comment: None,
                    number: 0,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if event.button == MouseButton::Middle {
                    return ToolUpdateResult::Unmodified;
                }

                if let Some(rectangle) = &mut self.rectangle {
                    rectangle.finishing = true;
                    if event.pos.x.abs() < 3.0 || event.pos.y.abs() < 3.0 {
                        self.rectangle = None;

                        ToolUpdateResult::Redraw
                    } else {
                        rectangle.calculate_shape(&event);
                        let result = rectangle.clone_box();
                        self.rectangle = None;
                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if event.button == MouseButton::Middle {
                    return ToolUpdateResult::Unmodified;
                }

                if let Some(rectangle) = &mut self.rectangle {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    rectangle.calculate_shape(&event);
                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.rectangle.is_some() {
            self.rectangle = None;
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.rectangle {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn get_tool_type(&self) -> super::Tools {
        Tools::Rectangle
    }

    fn set_sender(&mut self, sender: Sender<SketchBoardInput>) {
        self.sender = Some(sender);
    }
}
