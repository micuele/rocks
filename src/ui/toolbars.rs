use std::collections::HashMap;

use crate::{
    configuration::{APP_CONFIG, key_for_tool},
    tools::Tools,
};

use crate::icons::icon_names::custom as icons;
use gtk::ToggleButton;
use relm4::gtk::gio::SimpleAction;
use relm4::{
    actions::{ActionablePlus, RelmAction, RelmActionGroup},
    gtk::{Align, prelude::*},
    prelude::*,
};

pub struct ToolsToolbar {
    active_button: Option<ToggleButton>,
    tool_buttons: HashMap<Tools, ToggleButton>,
    tool_action: SimpleAction,
}

#[derive(Debug, Copy, Clone)]
pub enum ToolbarEvent {
    ToolSelected(Tools),
    Redo,
    Undo,
    SaveFile,
    CopyClipboard,
    ClearAll,
    SaveFileAs,
}

#[derive(Debug, Copy, Clone)]
pub enum ToolsToolbarInput {
    SwitchSelectedTool(Tools),
    SetToolEditing(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for ToolsToolbar {
    type Init = ();
    type Input = ToolsToolbarInput;
    type Output = ToolbarEvent;

    view! {
        root = gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 2,
            set_valign: Align::Start,
            set_halign: Align::Center,
            add_css_class: "toolbar",
            add_css_class: "toolbar-top",

            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::TRASH_SYMBOLIC,
                set_tooltip: "Clear all",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::ClearAll);},
            },
            gtk::Separator {},
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::UNDO_SYMBOLIC,
                set_tooltip: "Undo (Ctrl-Z)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::Undo);},
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::REDO_SYMBOLIC,
                set_tooltip: "Redo (Ctrl-Y)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::Redo);},
            },
            gtk::Separator {},
            #[name(brush_button)]
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::PEN_SYMBOLIC,
                // tooltip set programmatically
                ActionablePlus::set_action::<ToolsAction>: Tools::Brush,
            },
            #[name(line_button)]
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::LINE_SYMBOLIC,
                // tooltip set programmatically
                ActionablePlus::set_action::<ToolsAction>: Tools::Line,
            },
            #[name(arrow_button)]
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::ARROW_SYMBOLIC,
                // tooltip set programmatically
                ActionablePlus::set_action::<ToolsAction>: Tools::Arrow,
            },
            #[name(rectangle_button)]
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::RECTANGLE_SYMBOLIC,
                // tooltip set programmatically
                ActionablePlus::set_action::<ToolsAction>: Tools::Rectangle,
            },
            gtk::Separator {},
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::COPY_SYMBOLIC,
                set_tooltip: "Copy to clipboard (Ctrl+C)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::CopyClipboard);},
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::SAVE_SYMBOLIC,
                set_tooltip: "Save (Ctrl+S)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::SaveFile);},

                set_visible: APP_CONFIG.read().output_filename().is_some()
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: icons::SAVE_AS_SYMBOLIC,
                set_tooltip: "Save as (Ctrl+Shift+S)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::SaveFileAs);},
            },
        },
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            ToolsToolbarInput::SwitchSelectedTool(tool) => {
                // Change state of action, let GTK update the UI
                self.tool_action.change_state(&tool.to_variant());

                if let Some(button) = self.active_button.as_ref() {
                    button.remove_css_class("editing");
                }
                self.active_button = self.tool_buttons.get(&tool).cloned();
            }
            ToolsToolbarInput::SetToolEditing(editing) => {
                if let Some(button) = self.active_button.as_ref() {
                    if editing {
                        button.add_css_class("editing");
                    } else {
                        button.remove_css_class("editing");
                    }
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let sender_tmp: ComponentSender<ToolsToolbar> = sender.clone();
        let tool_action: RelmAction<ToolsAction> = RelmAction::new_stateful_with_target_value(
            &APP_CONFIG.read().initial_tool(),
            move |_, state, value| {
                *state = value;
                // notify parent of change
                sender_tmp
                    .output_sender()
                    .emit(ToolbarEvent::ToolSelected(*state));
                // also change tracked active button
                sender_tmp
                    .input_sender()
                    .emit(ToolsToolbarInput::SwitchSelectedTool(*state))
            },
        );

        let mut model = ToolsToolbar {
            active_button: None,
            tool_buttons: HashMap::new(),
            tool_action: tool_action.clone().into(),
        };
        let widgets = view_output!();

        model.tool_buttons = HashMap::from([
            (Tools::Brush, widgets.brush_button.clone()),
            (Tools::Line, widgets.line_button.clone()),
            (Tools::Arrow, widgets.arrow_button.clone()),
            (Tools::Rectangle, widgets.rectangle_button.clone()),
        ]);

        // Show the single-key shortcut in each tool tooltip
        for (tool, button) in &model.tool_buttons {
            let tooltip = match key_for_tool(*tool) {
                Some(key) => format!("{} ({})", tool.display_name(), key.to_ascii_uppercase()),
                None => tool.display_name().to_owned(),
            };
            button.set_tooltip_text(Some(&tooltip));
        }

        // Set initial active button correctly
        let initial_tool = APP_CONFIG.read().initial_tool();
        if let Some(button) = model.tool_buttons.get(&initial_tool) {
            model.active_button = Some(button.clone());
        }

        let mut group = RelmActionGroup::<ToolsToolbarActionGroup>::new();
        group.add_action(tool_action);
        group.register_for_widget(&widgets.root);

        ComponentParts { model, widgets }
    }
}

relm4::new_action_group!(ToolsToolbarActionGroup, "tools-toolbars");
relm4::new_stateful_action!(ToolsAction, ToolsToolbarActionGroup, "tools", Tools, Tools);
