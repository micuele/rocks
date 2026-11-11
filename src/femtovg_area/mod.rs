mod imp;

use std::{cell::RefCell, rc::Rc};

use gtk::glib;
use relm4::gtk::gdk_pixbuf::{Pixbuf, glib::subclass::types::ObjectSubclassIsExt};
use relm4::{
    Sender,
    gtk::{self, prelude::WidgetExt, subclass::prelude::GLAreaImpl},
};

use crate::{
    configuration::Action,
    math::Vec2D,
    sketch_board::SketchBoardInput,
    tools::{CropTool, Drawable, Tool},
};

glib::wrapper! {
    pub struct FemtoVGArea(ObjectSubclass<imp::FemtoVGArea>)
        @extends gtk::Widget, gtk::GLArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for FemtoVGArea {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl FemtoVGArea {
    pub fn comment_at(&self, pos: Vec2D) -> Option<(usize, Box<dyn Drawable>)> {
        let pos = self.abs_canvas_to_image_coordinates(pos);
        self.imp().inner().as_ref()?.comment_at(pos)
    }
    pub fn edit_comment(&mut self, index: usize, drawable: Box<dyn Drawable>) {
        self.imp()
            .inner()
            .as_mut()
            .expect("renderer initialized")
            .edit_comment(index, drawable);
    }
    pub fn region_comments(&self) -> Vec<String> {
        self.imp()
            .inner()
            .as_ref()
            .expect("renderer initialized")
            .region_comments()
    }
    pub fn set_active_tool(&mut self, active_tool: Rc<RefCell<dyn Tool>>) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_active_tool(active_tool);
    }

    pub fn commit(&mut self, drawable: Box<dyn Drawable>) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .commit(drawable);
    }
    pub fn undo(&mut self) -> bool {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .undo()
    }
    pub fn redo(&mut self) -> bool {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .redo()
    }
    pub fn request_render(&self, actions: &[Action]) {
        self.imp().request_render(actions);
    }
    pub fn clear_all(&mut self) -> bool {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .clear_all()
    }

    pub fn abs_canvas_to_image_coordinates(&self, input: Vec2D) -> Vec2D {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .abs_canvas_to_image_coordinates(input, self.scale_factor() as f32)
    }

    pub fn rel_canvas_to_image_coordinates(&self, input: Vec2D) -> Vec2D {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .rel_canvas_to_image_coordinates(input, self.scale_factor() as f32)
    }

    pub fn init(
        &mut self,
        sender: Sender<SketchBoardInput>,
        crop_tool: Rc<RefCell<CropTool>>,
        active_tool: Rc<RefCell<dyn Tool>>,
        background_image: Pixbuf,
    ) {
        self.imp()
            .init(sender, crop_tool, active_tool, background_image);
    }

    pub fn set_zoom_scale(&self, factor: f32) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_zoom_scale(factor, false);
        //trigger resize to recalculate zoom
        self.imp().resize(0, 0);
    }

    pub fn set_pointer_offset(&self, offset: Vec2D) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_pointer_offset(offset * self.scale_factor() as f32);
    }

    pub fn set_pointer_offset_center(&self) {
        let center = Vec2D::new(
            self.allocated_width() as f32 / 2.0,
            self.allocated_height() as f32 / 2.0,
        );
        self.set_pointer_offset(center);
    }

    pub fn set_drag_offset(&self, offset: Vec2D) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_drag_offset(offset * self.scale_factor() as f32);
        //trigger resize to recalculate offset
        self.imp().resize(0, 0);
    }

    pub fn store_last_offset(&self) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .store_last_offset();
    }

    pub fn set_is_drag(&self, is_drag: bool) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_is_drag(is_drag);
    }

    pub fn resize(&self, width: i32, height: i32) {
        self.imp().resize(width, height);
    }
}
