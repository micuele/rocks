use femtovg::imgref::Img;
use femtovg::rgb::{ComponentBytes, RGBA};
use keycode::{KeyMap, KeyMappingId};
use relm4::gtk::gdk_pixbuf::Pixbuf;
use relm4::gtk::gdk_pixbuf::glib::Bytes;
use std::cell::RefCell;
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;

use relm4::gtk::gdk::{Key, ModifierType, Texture};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt, gtk};

use crate::configuration::{APP_CONFIG, Action, tool_for_key};
use crate::femtovg_area::FemtoVGArea;
use crate::math::Vec2D;
use crate::notification::{log_result, log_result_with_pixbuf};
use crate::style::Style;
use crate::tools::{Tool, ToolEvent, ToolUpdateResult, Tools, ToolsManager};
use crate::ui::toolbars::ToolbarEvent;
use xdg::BaseDirectories;

type RenderedImage = Img<Vec<RGBA<u8>>>;
const SAVE_AS_LAST_DIR_FILE: &str = "save_as_last_dir";
const SAVE_AS_LAST_DIR_MAX_BYTES: u64 = 10_000;
const ZOOM_FACTOR: f32 = 1.1;
const PAN_STEP_SIZE: f32 = 50.0;

#[derive(Debug, Clone)]
pub enum SketchBoardInput {
    InputEvent(InputEvent),
    PinchStart,
    PinchScale(f32),
    PinchEnd,
    ToolbarEvent(ToolbarEvent),
    RenderResult(RenderedImage, Vec<Action>),
    RenderResultFollowup(Option<Pixbuf>, Vec<Action>),
    Refresh,
    RegionComment(Option<String>),
    Exit,
    ScaleFactorChanged,
    Output(SketchBoardOutput),
}

#[derive(Debug, Clone)]
pub enum SketchBoardOutput {
    ToolSwitchShortcut(Tools),
    ToolEditingChanged(bool),
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Mouse(MouseEventMsg),
    Key(KeyEventMsg),
    KeyRelease(KeyEventMsg),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum MouseButton {
    Primary,
    Secondary,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEventMsg {
    pub key: Key,
    pub code: u32,
    pub modifier: ModifierType,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEventType {
    BeginDrag,
    EndDrag,
    UpdateDrag,
    Click,
    Scroll,
    PointerPos,
    Release,
    //Motion(Vec2D),
}

#[derive(Debug, Clone, Copy)]
pub struct MouseEventMsg {
    pub type_: MouseEventType,
    pub button: MouseButton,
    pub modifier: ModifierType,
    pub screen_pos: Vec2D,
    pub is_touchpad: bool,
    pub pos: Vec2D,
    pub n_pressed: i32,
    pub release: bool,
}

impl SketchBoardInput {
    pub fn new_mouse_event(
        event_type: MouseEventType,
        button: u32,
        n_pressed: i32,
        modifier: ModifierType,
        pos: Vec2D,
        release: bool,
    ) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Mouse(MouseEventMsg {
            type_: event_type,
            button: button.into(),
            n_pressed,
            modifier,
            screen_pos: pos,
            pos,
            is_touchpad: false,
            release,
        }))
    }
    pub fn new_key_event(event: KeyEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Key(event))
    }

    pub fn new_key_release_event(event: KeyEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::KeyRelease(event))
    }

    pub fn new_scroll_event(
        delta_x: f64,
        delta_y: f64,
        modifier: ModifierType,
        is_touchpad: bool,
    ) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Mouse(MouseEventMsg {
            type_: MouseEventType::Scroll,
            button: MouseButton::Middle,
            n_pressed: 0,
            modifier,
            screen_pos: Vec2D::new(delta_x as f32, delta_y as f32),
            pos: Vec2D::new(delta_x as f32, delta_y as f32),
            is_touchpad,
            release: false,
        }))
    }
}

impl From<u32> for MouseButton {
    fn from(value: u32) -> Self {
        match value {
            gtk::gdk::BUTTON_PRIMARY => MouseButton::Primary,
            gtk::gdk::BUTTON_MIDDLE => MouseButton::Middle,
            gtk::gdk::BUTTON_SECONDARY => MouseButton::Secondary,
            _ => MouseButton::Primary,
        }
    }
}

impl InputEvent {
    fn handle_event_mouse_input(&mut self, renderer: &FemtoVGArea) -> Option<ToolUpdateResult> {
        if let InputEvent::Mouse(me) = self {
            match me.type_ {
                MouseEventType::Click => {
                    me.pos = renderer.abs_canvas_to_image_coordinates(me.pos);
                    None
                }
                MouseEventType::Release => {
                    me.pos = renderer.abs_canvas_to_image_coordinates(me.pos);
                    None
                }
                MouseEventType::BeginDrag => {
                    me.pos = renderer.abs_canvas_to_image_coordinates(me.pos);
                    None
                }
                MouseEventType::EndDrag | MouseEventType::UpdateDrag => {
                    me.pos = renderer.rel_canvas_to_image_coordinates(me.pos);
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn handle_mouse_event(&mut self, renderer: &FemtoVGArea) -> Option<ToolUpdateResult> {
        if let InputEvent::Mouse(me) = self {
            match me.type_ {
                MouseEventType::EndDrag | MouseEventType::UpdateDrag => {
                    if me.button == MouseButton::Middle {
                        renderer.set_drag_offset(me.screen_pos);
                        renderer.set_is_drag(true);

                        if me.type_ == MouseEventType::EndDrag {
                            renderer.store_last_offset();
                            renderer.set_is_drag(false);
                        }
                        renderer.request_render(&[]);
                    }
                    None
                }

                MouseEventType::Scroll => {
                    if me.modifier.contains(ModifierType::CONTROL_MASK) {
                        let mut factor = ZOOM_FACTOR;
                        if me.is_touchpad {
                            factor = 1.0 + (factor - 1.0) / 2.0; // decrease a bit
                        };
                        match me.pos.y {
                            v if v < 0.0 => renderer.set_zoom_scale(factor),
                            v if v > 0.0 => renderer.set_zoom_scale(1f32 / factor),
                            _ => {}
                        }
                    } else {
                        let pan_step_size = if me.is_touchpad {
                            -2.0 // invert for natural scrolling and decrease a bit
                        } else {
                            PAN_STEP_SIZE
                        };
                        renderer.set_drag_offset(me.screen_pos * pan_step_size);
                        renderer.store_last_offset();
                    }
                    renderer.request_render(&[]);
                    None
                }
                MouseEventType::PointerPos => {
                    renderer.set_pointer_offset(me.pos);
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

pub struct SketchBoard {
    renderer: FemtoVGArea,
    active_tool: Rc<RefCell<dyn Tool>>,
    tool_edit_mode: bool,
    tools: ToolsManager,
    pinch_last_scale: f32,
    style: Style,
    space_held: bool,
    pan_drag: bool,
    editing_region: Option<usize>,
    pending_region: Option<Box<dyn crate::tools::Drawable>>,
}

impl SketchBoard {
    fn prompt_region_comment(
        &mut self,
        drawable: Box<dyn crate::tools::Drawable>,
        sender: ComponentSender<Self>,
    ) {
        let initial_comment = drawable.region_comment().unwrap_or("").to_owned();
        let number = if self.editing_region.is_some() {
            drawable.region_number()
        } else {
            self.renderer.region_comments().len() + 1
        };
        self.pending_region = Some(drawable);
        let parent = self.renderer.root().and_downcast::<gtk::Window>();
        let dialog = gtk::Dialog::builder()
            .title(format!("Region {number} — add a comment"))
            .modal(true)
            .decorated(false)
            .default_width(520)
            .build();
        dialog.add_css_class("comment-dialog");
        dialog.set_transient_for(parent.as_ref());
        let content = dialog.content_area();
        content.add_css_class("comment-content");
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let heading = gtk::Label::new(Some(&format!("Comment ({number})")));
        heading.add_css_class("comment-heading");
        heading.set_hexpand(true);
        heading.set_xalign(0.0);
        header.append(&heading);
        let close = gtk::Button::from_icon_name("window-close-symbolic");
        close.add_css_class("comment-close");
        close.set_valign(gtk::Align::Start);
        close.set_tooltip_text(Some("Cancel (Esc)"));
        let weak = dialog.downgrade();
        close.connect_clicked(move |_| {
            if let Some(dialog) = weak.upgrade() {
                dialog.response(gtk::ResponseType::Cancel);
            }
        });
        header.append(&close);
        content.append(&header);

        let input = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .build();
        let scroll = gtk::ScrolledWindow::builder()
            .min_content_height(100)
            .max_content_height(260)
            .child(&input)
            .build();
        scroll.add_css_class("comment-input");
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scroll));
        let placeholder = gtk::Label::new(Some("What should change here?"));
        placeholder.add_css_class("comment-placeholder");
        placeholder.set_halign(gtk::Align::Start);
        placeholder.set_valign(gtk::Align::Start);
        placeholder.set_can_target(false);
        overlay.add_overlay(&placeholder);
        content.append(&overlay);

        let hint = gtk::Label::new(Some(
            "Esc to cancel · Enter to save · Shift+Enter for a new line",
        ));
        hint.add_css_class("comment-hint");
        hint.set_xalign(0.0);
        content.append(&hint);
        input.buffer().connect_changed(move |buffer| {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            placeholder.set_visible(text.is_empty());
        });
        input.buffer().set_text(&initial_comment);
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak_dialog = dialog.downgrade();
        keys.connect_key_pressed(move |_, key, _, modifiers| {
            if let Some(dialog) = weak_dialog.upgrade() {
                if (key == Key::Return || key == Key::KP_Enter)
                    && !modifiers.contains(ModifierType::SHIFT_MASK)
                {
                    dialog.response(gtk::ResponseType::Accept);
                    return gtk::glib::Propagation::Stop;
                }
                if key == Key::Escape {
                    dialog.response(gtk::ResponseType::Cancel);
                    return gtk::glib::Propagation::Stop;
                }
            }
            gtk::glib::Propagation::Proceed
        });
        input.add_controller(keys);
        let focus_input = input.clone();
        dialog.connect_response(move |dialog, response| {
            let buffer = input.buffer();
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();
            if response == gtk::ResponseType::Accept && text.trim().is_empty() {
                input.set_tooltip_text(Some("Type a comment, or cancel this region."));
                input.grab_focus();
                return;
            }
            sender.input(SketchBoardInput::RegionComment(
                (response == gtk::ResponseType::Accept).then(|| text.trim().to_owned()),
            ));
            dialog.close();
        });
        dialog.present();
        focus_input.grab_focus();
    }

    fn refresh_screen(&mut self) {
        self.renderer.queue_render();
    }

    fn image_to_pixbuf(image: RenderedImage) -> Pixbuf {
        let (buf, w, h) = image.into_contiguous_buf();

        Pixbuf::from_bytes(
            &Bytes::from(buf.as_bytes()),
            relm4::gtk::gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            w as i32,
            h as i32,
            w as i32 * 4,
        )
    }

    fn deactivate_active_tool(&mut self) -> bool {
        if self.active_tool.borrow().active()
            && let ToolUpdateResult::Commit(result) =
                self.active_tool.borrow_mut().handle_deactivated()
        {
            self.renderer.commit(result);
            return true;
        }
        false
    }

    fn handle_action(&mut self, actions: &[Action]) -> ToolUpdateResult {
        let rv = if self.deactivate_active_tool() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        };
        self.renderer.request_render(actions);
        rv
    }

    fn handle_render_result_with_pixbuf(
        &self,
        pix_buf: Option<Pixbuf>,
        actions: Vec<Action>,
        sender: ComponentSender<Self>,
    ) {
        let mut iter = actions.into_iter();
        let mut early_exit = false;
        while let Some(action) = iter.next() {
            match action {
                Action::SaveToClipboard => {
                    if let Some(ref pix_buf) = pix_buf {
                        if !self.handle_copy_clipboard(pix_buf) {
                            return;
                        }
                        early_exit = APP_CONFIG.read().early_exit_copy();
                    }
                }
                Action::SaveToFile => {
                    if let Some(ref pix_buf) = pix_buf {
                        self.handle_save(pix_buf);
                        early_exit = APP_CONFIG.read().early_exit_save();
                    }
                }
                /* SaveToFileAs runs through a callback, so any further actions need to be triggered
                from the callback rather than further iterating actions here */
                Action::SaveToFileAs => {
                    if let Some(pix_buf) = pix_buf {
                        let followup_actions: Vec<Action> = iter.collect();
                        let is_modal =
                            APP_CONFIG.read().early_exit_save_as() || !followup_actions.is_empty();
                        self.handle_save_as(is_modal, pix_buf, sender, followup_actions);
                    }
                    return;
                }
                Action::Exit => (),
            }

            if early_exit {
                log_result("Early exit, ignoring further actions.", false);
                self.handle_exit();
                return;
            }
            if action == Action::Exit {
                log_result("Exit action, ignoring further actions.", false);
                self.handle_exit();
                return;
            }
        }
    }

    fn handle_render_result(
        &self,
        image: RenderedImage,
        actions: Vec<Action>,
        sender: ComponentSender<Self>,
    ) {
        let needs_pixbuf = actions.iter().any(|action| {
            matches!(
                action,
                Action::SaveToClipboard | Action::SaveToFile | Action::SaveToFileAs
            )
        });

        let pix_buf = if needs_pixbuf {
            Some(Self::image_to_pixbuf(image))
        } else {
            None
        };

        self.handle_render_result_with_pixbuf(pix_buf, actions, sender);
    }

    fn handle_exit(&self) {
        relm4::main_application().quit();
    }

    fn resolve_output_filename(output_filename: &str) -> Option<String> {
        let delayed_format = chrono::Local::now().format(output_filename);
        let mut output_filename = if panic::catch_unwind(|| delayed_format.to_string()).is_ok() {
            delayed_format.to_string()
        } else {
            eprintln!(
                "Warning: Could not format filename {output_filename} due to chrono format error, falling back to literal filename."
            );
            output_filename.to_owned()
        };

        if let Some(tilde_stripped) =
            output_filename.strip_prefix(&format!("~{}", std::path::MAIN_SEPARATOR_STR))
        {
            if let Some(mut home_dir) = std::env::home_dir() {
                home_dir.push(tilde_stripped);
                output_filename = home_dir.to_string_lossy().into_owned();
            } else {
                log_result("~ found but could not determine homedir", true);
                return None;
            }
        }

        Some(output_filename)
    }

    fn configured_output_path() -> Option<PathBuf> {
        APP_CONFIG
            .read()
            .output_filename()
            .and_then(|output_filename| Self::resolve_output_filename(output_filename))
            .map(PathBuf::from)
    }

    fn save_as_last_dir_file() -> Option<PathBuf> {
        let dirs = BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"));
        dirs.get_state_file(SAVE_AS_LAST_DIR_FILE)
    }

    fn save_as_last_dir_file_for_write() -> Option<PathBuf> {
        let dirs = BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"));
        dirs.place_state_file(SAVE_AS_LAST_DIR_FILE).ok()
    }

    fn save_as_initial_dir(
        last_dir_file: Option<&Path>,
        configured_output_path: Option<&Path>,
    ) -> Option<PathBuf> {
        if let Some(last_dir_file) = last_dir_file
            && fs::metadata(last_dir_file).is_ok_and(|metadata| {
                metadata.is_file() && metadata.len() <= SAVE_AS_LAST_DIR_MAX_BYTES
            })
            && let Ok(last_dir) = fs::read_to_string(last_dir_file)
        {
            let last_dir = PathBuf::from(last_dir);
            if last_dir.is_dir() {
                return Some(last_dir);
            }
        }

        configured_output_path
            .and_then(Path::parent)
            .filter(|parent| parent.is_dir())
            .map(Path::to_path_buf)
    }

    fn remember_save_as_dir(output_filename: &Path) {
        let Some(last_dir_file) = Self::save_as_last_dir_file_for_write() else {
            return;
        };
        Self::write_save_as_last_dir(&last_dir_file, output_filename);
    }

    fn write_save_as_last_dir(last_dir_file: &Path, output_filename: &Path) {
        let Some(parent) = output_filename.parent() else {
            return;
        };

        let _ = fs::write(last_dir_file, parent.to_string_lossy().as_bytes());
    }

    fn handle_save(&self, image: &Pixbuf) {
        let output_filename = match APP_CONFIG.read().output_filename() {
            None => {
                eprintln!("No Output filename specified!");
                return;
            }
            Some(o) => o.clone(),
        };

        let Some(output_filename) = Self::resolve_output_filename(&output_filename) else {
            return;
        };

        if !output_filename.ends_with(".png") {
            log_result(
                "The only supported format is png, but the filename does not end in png",
                true,
            );
            return;
        }

        let data = match image.save_to_bufferv("png", &Vec::new()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error serializing image: {e}");
                return;
            }
        };

        match fs::write(&output_filename, data) {
            Err(e) => log_result(&format!("Error while saving file: {e}"), true),
            Ok(_) => log_result_with_pixbuf(
                &format!("File saved to '{}'.", output_filename),
                image.clone(),
            ),
        };
    }

    fn handle_save_as(
        &self,
        is_modal: bool,
        pixbuf: Pixbuf,
        sender: ComponentSender<Self>,
        followup_actions: Vec<Action>,
    ) {
        let configured_output_path = Self::configured_output_path();
        let initial_dir = Self::save_as_initial_dir(
            Self::save_as_last_dir_file().as_deref(),
            configured_output_path.as_deref(),
        );
        let suggested_filename = configured_output_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned());

        let data = match pixbuf.save_to_bufferv("png", &Vec::new()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error serializing image: {e}");
                return;
            }
        };

        let root = self.renderer.toplevel_window();

        relm4::spawn_local(async move {
            let builder = gtk::FileChooserNative::builder()
                .modal(is_modal)
                .title("Save Image As")
                .action(gtk::FileChooserAction::Save)
                .accept_label("Save")
                .cancel_label("Cancel");

            let dialog = match root {
                Some(w) => builder.transient_for(&w),
                None => builder,
            }
            .build();

            if let Some(initial_dir) = initial_dir {
                let initial_dir = gtk::gio::File::for_path(initial_dir);
                if let Err(e) = dialog.set_current_folder(Some(&initial_dir)) {
                    eprintln!("Error setting Save As folder: {e}");
                }
            }

            if let Some(filename) = suggested_filename {
                dialog.set_current_name(&filename);
            }

            dialog.connect_response(move |dialog, response| {
                let mut exit_app = false;
                let mut saved = false;
                if response == gtk::ResponseType::Accept
                    && let Some(file) = dialog.file()
                {
                    let output_filename = match file.path() {
                        Some(path) => path.to_string_lossy().into_owned(),
                        None => return,
                    };

                    match fs::write(&output_filename, &data) {
                        Err(e) => log_result(&format!("Error while saving file: {e}"), true),
                        Ok(_) => {
                            exit_app = APP_CONFIG.read().early_exit_save_as();
                            saved = true;
                            Self::remember_save_as_dir(Path::new(&output_filename));
                            log_result_with_pixbuf(
                                &format!("File saved to '{}'.", output_filename),
                                pixbuf.clone(),
                            )
                        }
                    };
                }
                if exit_app {
                    log_result("early exit after save as, ignoring further actions.", false);
                    sender.input(SketchBoardInput::Exit);
                } else if saved || !followup_actions.is_empty() {
                    sender.input(SketchBoardInput::RenderResultFollowup(
                        Some(pixbuf.clone()),
                        followup_actions.clone(),
                    ));
                }
            });

            dialog.show();
        });
    }

    fn handle_copy_clipboard(&self, image: &Pixbuf) -> bool {
        let texture = Texture::for_pixbuf(image);

        let comments = self.renderer.region_comments();
        match crate::clipboard::copy(&texture, &comments) {
            Err(e) => {
                eprintln!("Error saving {e}");
                false
            }
            Ok(()) => {
                log_result_with_pixbuf("Copied to clipboard.", image.clone());
                true
            }
        }
    }

    fn handle_undo(&mut self) -> ToolUpdateResult {
        if self.active_tool.borrow().active() {
            self.active_tool.borrow_mut().handle_undo()
        } else if self.renderer.undo() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_redo(&mut self) -> ToolUpdateResult {
        if self.active_tool.borrow().active() {
            self.active_tool.borrow_mut().handle_redo()
        } else if self.renderer.redo() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_clear_all(&mut self) -> ToolUpdateResult {
        // can't use lazy || here
        if self.deactivate_active_tool() | self.renderer.clear_all() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_toolbar_event(
        &mut self,
        toolbar_event: ToolbarEvent,
        sender: ComponentSender<Self>,
    ) -> ToolUpdateResult {
        match toolbar_event {
            ToolbarEvent::ToolSelected(tool) => {
                // deactivate old tool and save drawable, if any
                let old_tool = self.active_tool.clone();
                let mut deactivate_result =
                    old_tool.borrow_mut().handle_event(ToolEvent::Deactivated);

                if let ToolUpdateResult::Commit(d) = deactivate_result {
                    self.renderer.commit(d);
                    // we handle commit directly and "downgrade" to a simple redraw result
                    deactivate_result = ToolUpdateResult::Redraw;
                }

                // change active tool
                self.active_tool = self.tools.get(&tool);
                self.renderer.set_active_tool(self.active_tool.clone());

                // set sender for tool
                self.active_tool
                    .borrow_mut()
                    .set_sender(sender.input_sender().clone());

                // send style event
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style));

                // send activated event
                let activate_result = self
                    .active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::Activated);

                match activate_result {
                    ToolUpdateResult::Unmodified => deactivate_result,
                    _ => activate_result,
                }
            }
            ToolbarEvent::SaveFile => self.handle_action(&[Action::SaveToFile]),
            ToolbarEvent::CopyClipboard => self.handle_action(&[Action::SaveToClipboard]),
            ToolbarEvent::Undo => self.handle_undo(),
            ToolbarEvent::Redo => self.handle_redo(),
            ToolbarEvent::ClearAll => self.handle_clear_all(),
            ToolbarEvent::SaveFileAs => self.handle_action(&[Action::SaveToFileAs]),
        }
    }

    pub fn active_tool_type(&self) -> Tools {
        self.active_tool.borrow().get_tool_type()
    }
}

#[relm4::component(pub)]
impl Component for SketchBoard {
    type CommandOutput = ();
    type Input = SketchBoardInput;
    type Output = SketchBoardOutput;
    type Init = Pixbuf;

    view! {
        gtk::Box {
            add_css_class: "inner_box",
            #[local_ref]
            area -> FemtoVGArea {
                set_vexpand: true,
                set_hexpand: true,
                set_can_focus: true,
                set_focusable: true,
                grab_focus: (),

                add_controller = gtk::GestureDrag {
                        set_button: 0,
                        connect_drag_begin[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::BeginDrag,
                                controller.current_button(),
                                1,
                                controller.current_event_state(),
                                Vec2D::new(x as f32, y as f32),
                                false,
                            ));

                        },
                        connect_drag_update[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::UpdateDrag,
                                controller.current_button(),
                                1,
                                controller.current_event_state(),
                                Vec2D::new(x as f32, y as f32),
                                false,
                            ));
                        },
                        connect_drag_end[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::EndDrag,
                                controller.current_button(),
                                1,
                                controller.current_event_state(),
                                Vec2D::new(x as f32, y as f32),
                                false
                            ));
                        }
                },

                add_controller = gtk::GestureClick {
                    set_button: 0,
                    connect_pressed[sender] => move |controller, n_pressed, x, y| {
                        sender.input(SketchBoardInput::new_mouse_event(
                            MouseEventType::Click,
                            controller.current_button(),
                            n_pressed,
                            controller.current_event_state(),
                            Vec2D::new(x as f32, y as f32),
                            false,
                        ));
                    },
                    connect_released[sender] => move |controller, n_released, x, y| {
                        sender.input(SketchBoardInput::new_mouse_event(
                            MouseEventType::Release,
                            controller.current_button(),
                            n_released,
                            controller.current_event_state(),
                            Vec2D::new(x as f32, y as f32),
                            true,
                        ));
                    }
                },

                add_controller = gtk::EventControllerScroll{
                    set_flags: gtk::EventControllerScrollFlags::VERTICAL
                        | gtk::EventControllerScrollFlags::HORIZONTAL,
                    connect_scroll[sender] => move |controller, dx, dy| {
                        let is_touchpad = controller
                            .current_event()
                            .and_then(|event| event.device())
                            .is_some_and(|device| {
                                matches!(device.source(), gtk::gdk::InputSource::Touchpad)
                            });

                        sender.input(SketchBoardInput::new_scroll_event(
                            dx,
                            dy,
                            controller.current_event_state(),
                            is_touchpad,
                        ));
                        relm4::gtk::glib::Propagation::Stop
                    },
                },

                add_controller = gtk::GestureZoom {
                    connect_begin[sender] => move |_, _| {
                        sender.input(SketchBoardInput::PinchStart);
                    },
                    connect_scale_changed[sender] => move |_, scale| {
                        sender.input(SketchBoardInput::PinchScale(scale as f32));
                    },
                    connect_end[sender] => move |_, _| {
                        sender.input(SketchBoardInput::PinchEnd);
                    },
                },

                add_controller = gtk::EventControllerKey {
                    connect_key_pressed[sender] => move |_, key, code, modifier| {
                        sender.input(SketchBoardInput::new_key_event(KeyEventMsg::new(key, code, modifier)));
                        relm4::gtk::glib::Propagation::Stop
                    },

                    connect_key_released[sender] => move |_, key, code, modifier| {
                        sender.input(SketchBoardInput::new_key_release_event(KeyEventMsg::new(key, code, modifier)));
                    },
                },

                add_controller = gtk::EventControllerMotion {
                    connect_motion[sender] => move |controller, x, y| {
                        sender.input(SketchBoardInput::new_mouse_event(
                            MouseEventType::PointerPos,
                            0,
                            0,
                            controller.current_event_state(),
                            Vec2D::new(x as f32, y as f32),
                            false
                        ));
                    }
                }
            }
        },
    }

    fn update(&mut self, msg: SketchBoardInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        // handle resize ourselves, pass everything else to tool
        let sender_clone = sender.clone();
        let result = match msg {
            SketchBoardInput::InputEvent(mut ie) => {
                if let InputEvent::Mouse(me) = &ie
                    && me.type_ == MouseEventType::Click
                    && matches!(me.button, MouseButton::Primary)
                    && !self.space_held
                    && !self.active_tool.borrow().active()
                    && let Some((index, drawable)) = self.renderer.comment_at(me.pos)
                {
                    self.editing_region = Some(index);
                    self.prompt_region_comment(drawable, sender_clone);
                    return;
                }
                if matches!(&ie, InputEvent::Key(ke) | InputEvent::KeyRelease(ke) if ke.key == Key::space)
                    && !self.active_tool.borrow().active()
                {
                    self.space_held = matches!(&ie, InputEvent::Key(_));
                    self.renderer.set_cursor_from_name(Some(if self.space_held {
                        "grab"
                    } else {
                        "default"
                    }));
                    return;
                }
                if let InputEvent::Mouse(me) = &mut ie {
                    if me.type_ == MouseEventType::BeginDrag
                        && me.button == MouseButton::Primary
                        && self.space_held
                    {
                        self.pan_drag = true;
                    }
                    if self.pan_drag {
                        me.button = MouseButton::Middle;
                        ie.handle_mouse_event(&self.renderer);
                        if matches!(&ie, InputEvent::Mouse(me) if me.type_ == MouseEventType::EndDrag)
                        {
                            self.pan_drag = false;
                        }
                        return;
                    }
                }
                if matches!(&ie, InputEvent::Key(ke) if ke.key == Key::Escape && ke.modifier.is_empty())
                    && self.active_tool_type() != Tools::Crop
                    && !self.active_tool.borrow().active()
                    && self.tools.get_crop_tool().borrow().get_crop().is_some()
                {
                    self.tools.get_crop_tool().borrow_mut().handle_dismissed();
                    self.refresh_screen();
                    return;
                }
                if let InputEvent::Key(ke) = ie {
                    let active_tool_result = self
                        .active_tool
                        .borrow_mut()
                        .handle_event(ToolEvent::Input(ie.clone()));

                    // eprintln!("active_tool_result={:?}", active_tool_result);

                    match active_tool_result {
                        ToolUpdateResult::StopPropagation
                        | ToolUpdateResult::RedrawAndStopPropagation => active_tool_result,
                        _ => {
                            if ke.key == Key::y && ke.modifier == ModifierType::CONTROL_MASK {
                                self.handle_redo()
                            } else if ke.is_one_of(Key::z, KeyMappingId::UsZ)
                                && ke.modifier == ModifierType::CONTROL_MASK
                            {
                                self.handle_undo()
                            } else if ke.is_one_of(Key::y, KeyMappingId::UsY)
                                && ke.modifier == ModifierType::CONTROL_MASK
                            {
                                self.handle_redo()
                            } else if ke.is_one_of(Key::s, KeyMappingId::UsS)
                                && ke.modifier == ModifierType::CONTROL_MASK
                            {
                                self.renderer.request_render(&[Action::SaveToFile]);
                                ToolUpdateResult::Unmodified
                            } else if ke.is_one_of(Key::s, KeyMappingId::UsS)
                                && ke.modifier
                                    == (ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK)
                            {
                                self.renderer.request_render(&[Action::SaveToFileAs]);
                                ToolUpdateResult::Unmodified
                            } else if ke.is_one_of(Key::c, KeyMappingId::UsC)
                                && ke.modifier == ModifierType::CONTROL_MASK
                            {
                                self.renderer.request_render(&[Action::SaveToClipboard]);
                                ToolUpdateResult::Unmodified
                            } else if (ke.is_one_of(Key::leftarrow, KeyMappingId::ArrowLeft)
                                || ke.is_one_of(Key::rightarrow, KeyMappingId::ArrowRight)
                                || ke.is_one_of(Key::uparrow, KeyMappingId::ArrowUp)
                                || ke.is_one_of(Key::downarrow, KeyMappingId::ArrowDown))
                                && ke.modifier == ModifierType::ALT_MASK
                            {
                                match ke.key {
                                    Key::Left => self
                                        .renderer
                                        .set_drag_offset(Vec2D::new(-PAN_STEP_SIZE, 0.)),
                                    Key::Right => {
                                        self.renderer.set_drag_offset(Vec2D::new(PAN_STEP_SIZE, 0.))
                                    }
                                    Key::Up => self
                                        .renderer
                                        .set_drag_offset(Vec2D::new(0., -PAN_STEP_SIZE)),
                                    Key::Down => {
                                        self.renderer.set_drag_offset(Vec2D::new(0., PAN_STEP_SIZE))
                                    }
                                    _ => { /* unreachable */ }
                                }

                                self.renderer.store_last_offset();
                                self.renderer.request_render(&[]);
                                ToolUpdateResult::Unmodified
                            } else if ke.modifier.is_empty() && ke.key == Key::Delete {
                                self.handle_clear_all()
                            } else if ke.modifier.is_empty()
                                && let Some(tool) = ke.key.to_unicode().and_then(tool_for_key)
                            {
                                sender.input(SketchBoardInput::ToolbarEvent(
                                    ToolbarEvent::ToolSelected(tool),
                                ));
                                sender
                                    .output_sender()
                                    .emit(SketchBoardOutput::ToolSwitchShortcut(tool));
                                ToolUpdateResult::Unmodified
                            } else if ke.modifier.is_empty()
                                && (ke.key == Key::Escape
                                    || ke.key == Key::Return
                                    || ke.key == Key::KP_Enter)
                            {
                                // First, let the tool handle the event. If the tool does nothing, we can do our thing (otherwise require a second keyboard press)
                                // Relying on ToolUpdateResult::Unmodified is probably not a good idea, but it's the only way at the moment. See discussion in #144
                                if let ToolUpdateResult::Unmodified = active_tool_result {
                                    let actions = if ke.key == Key::Escape {
                                        APP_CONFIG.read().actions_on_escape()
                                    } else {
                                        APP_CONFIG.read().actions_on_enter()
                                    };
                                    self.renderer.request_render(&actions);
                                };
                                active_tool_result
                            } else {
                                active_tool_result
                            }
                        }
                    }
                } else {
                    if let InputEvent::Mouse(me) = &ie
                        && matches!(me.type_, MouseEventType::Click | MouseEventType::BeginDrag)
                    {
                        self.renderer.grab_focus();
                    }

                    ie.handle_event_mouse_input(&self.renderer);
                    // Right-drag edits the crop without changing the selected drawing tool.
                    let active_tool_result = if let InputEvent::Mouse(me) = &ie
                        && me.button == MouseButton::Secondary
                    {
                        let mut crop_event = me.clone();
                        crop_event.button = MouseButton::Primary;
                        crop_event.modifier.remove(ModifierType::CONTROL_MASK);
                        self.tools
                            .get_crop_tool()
                            .borrow_mut()
                            .handle_mouse_event(crop_event)
                    } else {
                        self.active_tool
                            .borrow_mut()
                            .handle_event(ToolEvent::Input(ie.clone()))
                    };

                    // eprintln!("active_tool_result={:?}", active_tool_result);

                    match active_tool_result {
                        ToolUpdateResult::StopPropagation
                        | ToolUpdateResult::RedrawAndStopPropagation => active_tool_result,
                        _ => {
                            if let Some(result) = ie.handle_mouse_event(&self.renderer) {
                                result
                            } else {
                                active_tool_result
                            }
                        }
                    }
                }
            }
            SketchBoardInput::PinchStart => {
                self.pinch_last_scale = 1.0;
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::PinchScale(scale) => {
                if scale.is_finite() && scale > 0.0 && self.pinch_last_scale > 0.0 {
                    let factor = scale / self.pinch_last_scale;
                    if factor.is_finite() && factor > 0.0 {
                        self.renderer.set_pointer_offset_center();
                        self.renderer.set_zoom_scale(factor);
                        self.renderer.request_render(&[]);
                    }
                    self.pinch_last_scale = scale;
                }
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::PinchEnd => {
                self.pinch_last_scale = 1.0;
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::ToolbarEvent(toolbar_event) => {
                self.handle_toolbar_event(toolbar_event, sender)
            }
            SketchBoardInput::RenderResult(img, action) => {
                self.handle_render_result(img, action, sender);
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::RenderResultFollowup(pix_buf, action) => {
                self.handle_render_result_with_pixbuf(pix_buf, action, sender);
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::RegionComment(comment) => {
                self.renderer.grab_focus();
                if let Some(mut drawable) = self.pending_region.take()
                    && let Some(comment) = comment
                {
                    drawable.set_region_comment(self.renderer.region_comments().len() + 1, comment);
                    if let Some(index) = self.editing_region.take() {
                        self.renderer.edit_comment(index, drawable);
                    } else {
                        self.renderer.commit(drawable);
                    }
                }
                self.editing_region = None;
                self.renderer.grab_focus();
                ToolUpdateResult::Redraw
            }
            SketchBoardInput::Refresh => ToolUpdateResult::Redraw,
            SketchBoardInput::Exit => {
                self.handle_exit();
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::ScaleFactorChanged => {
                self.renderer.resize(0, 0);
                ToolUpdateResult::Redraw
            }
            SketchBoardInput::Output(output) => {
                sender.output_sender().emit(output);
                ToolUpdateResult::Unmodified
            }
        };

        let editing = self.active_tool.borrow().active();
        if editing != self.tool_edit_mode {
            self.tool_edit_mode = editing;
            sender_clone
                .output_sender()
                .emit(SketchBoardOutput::ToolEditingChanged(editing));
        }

        match result {
            ToolUpdateResult::Commit(drawable) => {
                if drawable.needs_comment() {
                    self.prompt_region_comment(drawable, sender_clone);
                    self.refresh_screen();
                    return;
                }
                self.renderer.commit(drawable);
                self.refresh_screen();
            }
            ToolUpdateResult::Unmodified | ToolUpdateResult::StopPropagation => (),
            ToolUpdateResult::Redraw | ToolUpdateResult::RedrawAndStopPropagation => {
                self.refresh_screen()
            }
        };
    }

    fn init(
        image: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = APP_CONFIG.read();
        let tools = ToolsManager::new();

        let mut model = Self {
            renderer: FemtoVGArea::default(),
            active_tool: tools.get(&config.initial_tool()),
            tool_edit_mode: false,
            style: Style::default(),
            pinch_last_scale: 1.0,
            tools,
            space_held: false,
            pan_drag: false,
            pending_region: None,
            editing_region: None,
        };

        let area = &mut model.renderer;
        area.init(
            sender.input_sender().clone(),
            model.tools.get_crop_tool(),
            model.active_tool.clone(),
            image,
        );

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl KeyEventMsg {
    pub fn new(key: Key, code: u32, modifier: ModifierType) -> Self {
        Self {
            key,
            code,
            modifier,
        }
    }

    /// Matches one of providen keys. The modifier is not considered.
    /// And the key has more priority over keycode.
    fn is_one_of(&self, key: Key, code: KeyMappingId) -> bool {
        // INFO: on linux the keycode from gtk4 is evdev keycode, so need to match by him if need
        // to use layout-independent shortcuts. And notice that there is subtraction by 8, it's
        // because of x11 compatibility in which the keycodes are in range [8,255]. So need shift
        // them to get correct evdev keycode.
        let keymap = KeyMap::from(code);
        self.key == key || self.code as u16 - 8 == keymap.evdev
    }
}

#[cfg(test)]
mod tests {
    use super::SketchBoard;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("rocks-{name}-{nanos}"));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn save_as_initial_dir_uses_remembered_existing_directory() {
        let temp = TempDir::new("remembered-dir");
        let remembered_dir = temp.path().join("remembered");
        let fallback_dir = temp.path().join("fallback");
        fs::create_dir_all(&remembered_dir).expect("create remembered dir");
        fs::create_dir_all(&fallback_dir).expect("create fallback dir");

        let state_file = temp.path().join("state").join("save_as_last_dir");
        fs::create_dir_all(state_file.parent().expect("state parent")).expect("create state dir");
        fs::write(&state_file, remembered_dir.to_string_lossy().as_bytes())
            .expect("write state file");

        let initial_dir = SketchBoard::save_as_initial_dir(
            Some(&state_file),
            Some(&fallback_dir.join("screenshot.png")),
        );

        assert_eq!(initial_dir, Some(remembered_dir));
    }

    #[test]
    fn save_as_initial_dir_falls_back_when_remembered_directory_is_invalid() {
        let temp = TempDir::new("invalid-remembered-dir");
        let fallback_dir = temp.path().join("fallback");
        fs::create_dir_all(&fallback_dir).expect("create fallback dir");

        let state_file = temp.path().join("save_as_last_dir");
        fs::write(
            &state_file,
            temp.path().join("missing").to_string_lossy().as_bytes(),
        )
        .expect("write invalid state file");

        let initial_dir = SketchBoard::save_as_initial_dir(
            Some(&state_file),
            Some(&fallback_dir.join("screenshot.png")),
        );

        assert_eq!(initial_dir, Some(fallback_dir));
    }

    #[test]
    fn save_as_initial_dir_handles_missing_state_and_output_path() {
        let initial_dir = SketchBoard::save_as_initial_dir(None, None);

        assert_eq!(initial_dir, None);
    }

    #[test]
    fn remember_save_as_dir_creates_state_file() {
        let temp = TempDir::new("remember-save-as-dir");
        let saved_dir = temp.path().join("saved");
        fs::create_dir_all(&saved_dir).expect("create saved dir");
        let state_dir = temp.path().join("state");
        fs::create_dir_all(&state_dir).expect("create state dir");
        let state_file = state_dir.join("save_as_last_dir");

        SketchBoard::write_save_as_last_dir(&state_file, &saved_dir.join("image.png"));

        let remembered_dir = fs::read_to_string(state_file).expect("read state file");
        assert_eq!(remembered_dir, saved_dir.to_string_lossy());
    }

    #[test]
    fn write_save_as_last_dir_ignores_unwritable_state_path() {
        let temp = TempDir::new("unwritable-state-path");
        let saved_dir = temp.path().join("saved");
        fs::create_dir_all(&saved_dir).expect("create saved dir");

        SketchBoard::write_save_as_last_dir(temp.path(), &saved_dir.join("image.png"));
    }
}
