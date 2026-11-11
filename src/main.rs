use configuration::{APP_CONFIG, Configuration};
use relm4::gtk::gdk_pixbuf::Pixbuf;
use relm4::gtk::gio::ApplicationFlags;
use relm4::gtk::prelude::*;
use std::ptr;
use std::sync::{LazyLock, RwLock};

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{self, CssProvider, Window, gdk::DisplayManager, gdk::FullscreenMode, gdk::Toplevel},
};

use anyhow::{Context, Result};
use command_line::Fullscreen;
use tempfile::TempDir;

use sketch_board::SketchBoardOutput;
use ui::toolbars::{ToolsToolbar, ToolsToolbarInput};

mod auto_crop;
mod clipboard;
mod command_line;
mod configuration;
mod femtovg_area;
mod icons;
mod math;
mod notification;
mod sketch_board;
mod style;
mod tools;
mod ui;

use crate::sketch_board::{SketchBoard, SketchBoardInput};
use crate::tools::Tools;

const APP_ID: &str = "rocks.searching.Rocks";

pub static TEMP_DIR: LazyLock<RwLock<Option<TempDir>>> =
    LazyLock::new(|| match tempfile::TempDir::new() {
        Ok(dir) => RwLock::new(Some(dir)),
        Err(e) => {
            eprintln!("Failed to create temporary directory: {}", e);
            RwLock::new(None)
        }
    });

struct App {
    image_dimensions: (i32, i32),
    sketch_board: Controller<SketchBoard>,
    tools_toolbar: Controller<ToolsToolbar>,
    outer_box: gtk::Box,
    overlay: gtk::Overlay,
}

#[derive(Debug)]
enum AppInput {
    Realized,
    ToolSwitchShortcut(Tools),
    ScaleFactorChanged,
    FullscreenChanged(bool),
    ToolEditingChanged(bool),
}

impl App {
    fn resize_window_initial(&self, root: &Window) {
        let fullscreen = APP_CONFIG.read().fullscreen();

        if fullscreen == Some(Fullscreen::All)
            && let Some(surface) = root.surface()
            && let Ok(toplevel) = surface.downcast::<Toplevel>()
        {
            toplevel.set_fullscreen_mode(FullscreenMode::AllMonitors);
        }

        root.set_default_size(self.image_dimensions.0, self.image_dimensions.1);

        if fullscreen.is_some() {
            root.fullscreen();
        }
    }

    fn apply_style() {
        let css_provider = CssProvider::new();
        css_provider.load_from_data(include_str!("assets/default.css"));

        match DisplayManager::get().default_display() {
            Some(display) => gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            ),
            None => eprintln!("Cannot apply style"),
        }
    }
}

#[relm4::component]
impl Component for App {
    type Init = Pixbuf;
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        main_window = gtk::Window {
            set_icon_name: Some(APP_ID),
            set_default_size: (500, 500),
            add_css_class: "root",
            set_title: Some(APP_CONFIG.read().title().map_or("rocks", |s| s.as_str())),

            #[local_ref]
            outer_box_clone -> gtk::Box {
                add_css_class: "outer_box",
                append = model.tools_toolbar.widget(),
                #[local_ref]
                overlay_clone -> gtk::Overlay {
                    add_css_class: "overlay",
                    model.sketch_board.widget(),
                },
            },

            connect_show[sender] => move |_| {
                sender.input(AppInput::Realized);
            },
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AppInput::Realized => self.resize_window_initial(root),
            AppInput::ToolSwitchShortcut(tool) => {
                self.tools_toolbar
                    .sender()
                    .emit(ToolsToolbarInput::SwitchSelectedTool(tool));
            }
            AppInput::ScaleFactorChanged => {
                self.sketch_board
                    .sender()
                    .emit(SketchBoardInput::ScaleFactorChanged);
            }
            AppInput::FullscreenChanged(fullscreen) => {
                let tools = self.tools_toolbar.widget();
                if fullscreen {
                    self.outer_box.remove(tools);
                    self.overlay.add_overlay(tools);
                } else {
                    self.overlay.remove_overlay(tools);
                    self.outer_box.prepend(tools);
                }
            }
            AppInput::ToolEditingChanged(editing) => {
                self.tools_toolbar
                    .sender()
                    .emit(ToolsToolbarInput::SetToolEditing(editing));
            }
        }
    }

    fn init(
        image: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        Self::apply_style();
        let image_dimensions = (image.width(), image.height());

        // SketchBoard
        let sketch_board =
            SketchBoard::builder()
                .launch(image)
                .forward(sender.input_sender(), |t| match t {
                    SketchBoardOutput::ToolSwitchShortcut(tool) => {
                        AppInput::ToolSwitchShortcut(tool)
                    }
                    SketchBoardOutput::ToolEditingChanged(editing) => {
                        AppInput::ToolEditingChanged(editing)
                    }
                });

        // Toolbars
        let tools_toolbar = ToolsToolbar::builder()
            .launch(())
            .forward(sketch_board.sender(), SketchBoardInput::ToolbarEvent);

        let outer_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let outer_box_clone = outer_box.clone();
        let overlay = gtk::Overlay::new();
        let overlay_clone = overlay.clone();

        // Model
        let model = App {
            sketch_board,
            tools_toolbar,
            image_dimensions,
            outer_box,
            overlay,
        };

        let widgets = view_output!();

        let sender_clone = sender.clone();
        root.connect_map(move |r| {
            let sender_clone = sender_clone.clone();
            if let Some(surface) = r.surface() {
                surface.connect_notify_local(Some("scale-factor"), move |_, _| {
                    sender_clone.input(AppInput::ScaleFactorChanged);
                });
            }
        });

        let sender_clone = sender.clone();
        root.connect_notify(Some("fullscreened"), move |window, _| {
            sender_clone.input(AppInput::FullscreenChanged(window.is_fullscreen()));
        });

        ComponentParts { model, widgets }
    }
}

fn load_gl() -> Result<()> {
    // Load GL pointers from epoxy (GL context management library used by GTK).
    let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }?;

    epoxy::load_with(|name| {
        unsafe { library.get::<_>(name.as_bytes()) }
            .map(|symbol| *symbol)
            .unwrap_or(ptr::null())
    });

    Ok(())
}

fn run_rocks() -> Result<()> {
    load_gl()?;

    let image =
        Pixbuf::from_file(APP_CONFIG.read().input_filename()).context("couldn't load image")?;

    // start GUI
    let app = relm4::main_application();
    app.set_application_id(Some(APP_ID));
    // set flag to allow to run multiple instances
    app.set_flags(ApplicationFlags::NON_UNIQUE);
    let app = RelmApp::from_app(app).with_args(vec![]);
    relm4_icons::initialize_icons(
        icons::icon_names::GRESOURCE_BYTES,
        icons::icon_names::RESOURCE_PREFIX,
    );

    app.run::<App>(image);

    match TEMP_DIR.write() {
        Ok(mut temp_dir) => {
            // take is sufficient to have the dir deleted when it's dropped.
            // But that would hide any errors, so use explicit close instead.
            if let Some(dir) = temp_dir.take()
                && let Err(e) = dir.close()
            {
                eprintln!("Failed to close temporary directory: {}", e);
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--clipboard-owner") {
        return clipboard::serve();
    }
    // populate the APP_CONFIG from commandline and
    // config file. this might exit, if an error occurred.
    Configuration::load();

    run_rocks().inspect_err(|e| eprintln!("Error: {e}"))
}
