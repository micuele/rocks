use std::{borrow::Cow, cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use anyhow::Result;
use femtovg::{Canvas, FontId, renderer::OpenGl};
use relm4::gtk::gdk_pixbuf::{
    glib::{Variant, VariantTy},
    prelude::{StaticVariantType, ToVariant},
};

use relm4::Sender;
use relm4::gtk::glib::variant::FromVariant;
use serde::Deserialize;

use crate::{
    math::Vec2D,
    sketch_board::{InputEvent, KeyEventMsg, MouseEventMsg, SketchBoardInput},
    style::Style,
};

use crate::command_line;

mod arrow;
mod brush;
mod crop;
mod drag_box;
mod line;
mod rectangle;

pub enum ToolEvent {
    Activated,
    Deactivated,
    Dismissed,
    Input(InputEvent),
    StyleChanged(Style),
}

pub trait Tool {
    fn handle_event(&mut self, event: ToolEvent) -> ToolUpdateResult {
        match event {
            ToolEvent::Activated => self.handle_activated(),
            ToolEvent::Deactivated => self.handle_deactivated(),
            ToolEvent::Dismissed => self.handle_dismissed(),
            ToolEvent::Input(e) => self.handle_input_event(e),
            ToolEvent::StyleChanged(s) => self.handle_style_event(s),
        }
    }

    fn handle_activated(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn handle_deactivated(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn handle_dismissed(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn handle_input_event(&mut self, event: InputEvent) -> ToolUpdateResult {
        match event {
            InputEvent::Mouse(e) => self.handle_mouse_event(e),
            InputEvent::Key(e) => self.handle_key_event(e),
            InputEvent::KeyRelease(e) => self.handle_key_release_event(e),
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        let _ = event;
        ToolUpdateResult::Unmodified
    }

    fn handle_key_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        let _ = event;
        ToolUpdateResult::Unmodified
    }

    fn handle_key_release_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        let _ = event;
        ToolUpdateResult::Unmodified
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        let _ = style;
        ToolUpdateResult::Unmodified
    }

    fn active(&self) -> bool {
        false
    }

    fn handle_undo(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn handle_redo(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn get_drawable(&self) -> Option<&dyn Drawable>;

    fn get_tool_type(&self) -> Tools;

    fn set_sender(&mut self, sender: Sender<SketchBoardInput>);
}

// the clone method below has been adapted from: https://stackoverflow.com/questions/30353462/how-to-clone-a-struct-storing-a-boxed-trait-object
// it feels "strange" and especially the fact that drawable has to derive from DrawableClone feels "wrong".
pub trait DrawableClone {
    fn clone_box(&self) -> Box<dyn Drawable>;
}

impl<T> DrawableClone for T
where
    T: 'static + Drawable + Clone,
{
    fn clone_box(&self) -> Box<dyn Drawable> {
        Box::new(self.clone())
    }
}

pub trait Drawable: DrawableClone + Debug {
    fn needs_comment(&self) -> bool {
        false
    }
    fn region_comment(&self) -> Option<&str> {
        None
    }
    fn set_region_comment(&mut self, _number: usize, _comment: String) {}
    fn region_number(&self) -> usize {
        0
    }
    fn comment_badge(&self, _bounds: (Vec2D, Vec2D)) -> Option<Vec2D> {
        None
    }
    fn set_region_number(&mut self, _number: usize) {}
    fn draw(&self, canvas: &mut Canvas<OpenGl>, font: FontId, bounds: (Vec2D, Vec2D))
    -> Result<()>;
    fn handle_undo(&mut self) {}
    fn handle_redo(&mut self) {}
}

#[derive(Debug)]
pub enum ToolUpdateResult {
    Commit(Box<dyn Drawable>),
    Redraw,
    Unmodified,
    StopPropagation,
    RedrawAndStopPropagation,
}

pub use arrow::ArrowTool;
pub use crop::CropTool;
pub use line::LineTool;
pub use rectangle::RectangleTool;

use self::brush::BrushTool;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tools {
    Crop = 1,
    Line = 2,
    Arrow = 3,
    Rectangle = 4,
    Brush = 10,
}

impl Tools {
    pub fn display_name(&self) -> &'static str {
        match self {
            Tools::Crop => "Crop",
            Tools::Brush => "Brush",
            Tools::Line => "Line",
            Tools::Arrow => "Arrow",
            Tools::Rectangle => "Rectangle",
        }
    }
}

pub struct ToolsManager {
    tools: HashMap<Tools, Rc<RefCell<dyn Tool>>>,
    crop_tool: Rc<RefCell<CropTool>>,
}

impl ToolsManager {
    pub fn new() -> Self {
        let mut tools: HashMap<Tools, Rc<RefCell<dyn Tool>>> = HashMap::new();
        tools.insert(Tools::Line, Rc::new(RefCell::new(LineTool::default())));
        tools.insert(Tools::Arrow, Rc::new(RefCell::new(ArrowTool::default())));
        tools.insert(
            Tools::Rectangle,
            Rc::new(RefCell::new(RectangleTool::default())),
        );
        tools.insert(Tools::Brush, Rc::new(RefCell::new(BrushTool::default())));

        let crop_tool = Rc::new(RefCell::new(CropTool::default()));
        Self { tools, crop_tool }
    }

    pub fn get(&self, tool: &Tools) -> Rc<RefCell<dyn Tool>> {
        match tool {
            Tools::Crop => self.crop_tool.clone(),
            _ => self.tools[tool].clone(),
        }
    }

    pub fn get_crop_tool(&self) -> Rc<RefCell<CropTool>> {
        self.crop_tool.clone()
    }
}

impl StaticVariantType for Tools {
    fn static_variant_type() -> Cow<'static, VariantTy> {
        Cow::Borrowed(VariantTy::UINT32)
    }
}

impl ToVariant for Tools {
    fn to_variant(&self) -> Variant {
        Variant::from(*self as u32)
    }
}

impl FromVariant for Tools {
    fn from_variant(variant: &Variant) -> Option<Self> {
        variant.get::<u32>().and_then(|v| match v {
            1 => Some(Tools::Crop),
            2 => Some(Tools::Line),
            3 => Some(Tools::Arrow),
            4 => Some(Tools::Rectangle),
            10 => Some(Tools::Brush),
            _ => None,
        })
    }
}

impl From<command_line::Tools> for Tools {
    fn from(tool: command_line::Tools) -> Self {
        match tool {
            command_line::Tools::Line => Self::Line,
            command_line::Tools::Arrow => Self::Arrow,
            command_line::Tools::Rectangle => Self::Rectangle,
            command_line::Tools::Brush => Self::Brush,
        }
    }
}
