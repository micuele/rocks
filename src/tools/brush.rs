use femtovg::{FontId, LineCap, LineJoin, Paint, Path};

use crate::{
    math::Vec2D,
    sketch_board::{MouseButton, MouseEventMsg, MouseEventType, SketchBoardInput},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult, Tools};
use relm4::Sender;

#[derive(Default)]
pub struct BrushTool {
    drawable: Option<BrushDrawable>,
    style: Style,
    sender: Option<Sender<SketchBoardInput>>,
}

#[derive(Debug, Clone)]
pub struct BrushDrawable {
    // The start point of the brush stroke this is relative to canvas
    // after this the points are relative to the start point
    start_point: Option<Vec2D>,
    points: Vec<Vec2D>,
    style: Style,
}

impl BrushDrawable {
    fn add_point(&mut self, point: Vec2D) {
        self.points.push(point);
    }
}

impl Drawable for BrushDrawable {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
        _bounds: (Vec2D, Vec2D),
    ) -> anyhow::Result<()> {
        if self.points.is_empty() {
            return Ok(());
        }

        let Some(start_point) = self.start_point else {
            return Ok(());
        };

        canvas.save();
        let mut path = Path::new();

        path.move_to(start_point.x, start_point.y);
        for p in self.points.iter().skip(1) {
            path.line_to(start_point.x + p.x, start_point.y + p.y);
        }

        canvas.stroke_path(
            &path,
            &Paint::from(self.style)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round),
        );
        canvas.restore();
        Ok(())
    }
}

impl Tool for BrushTool {
    fn get_tool_type(&self) -> super::Tools {
        Tools::Brush
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                if event.button == MouseButton::Middle {
                    return ToolUpdateResult::Unmodified;
                }

                let Some(brush) = &mut self.drawable else {
                    return ToolUpdateResult::Unmodified;
                };
                brush.start_point = Some(event.pos);
                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if event.button == MouseButton::Middle {
                    return ToolUpdateResult::Unmodified;
                }

                let Some(brush) = &mut self.drawable else {
                    return ToolUpdateResult::Unmodified;
                };
                brush.add_point(event.pos);

                // commit
                let result = brush.clone_box();
                self.drawable = None;

                ToolUpdateResult::Commit(result)
            }
            MouseEventType::UpdateDrag => {
                if event.button == MouseButton::Middle {
                    return ToolUpdateResult::Unmodified;
                }

                let Some(brush) = &mut self.drawable else {
                    return ToolUpdateResult::Unmodified;
                };
                brush.add_point(event.pos);
                ToolUpdateResult::Redraw
            }
            MouseEventType::Click => {
                if event.button != MouseButton::Primary {
                    return ToolUpdateResult::Unmodified;
                }
                self.drawable = Some(BrushDrawable {
                    start_point: None,
                    points: vec![event.pos],
                    style: self.style,
                });
                ToolUpdateResult::Unmodified
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.drawable {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }

    fn active(&self) -> bool {
        self.drawable.is_some()
    }

    fn set_sender(&mut self, sender: Sender<SketchBoardInput>) {
        self.sender = Some(sender);
    }
}
