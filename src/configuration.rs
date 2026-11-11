use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use clap::Parser;
use relm4::SharedState;

use serde::Deserialize;
use serde::de::Deserializer;
use thiserror::Error;
use xdg::{BaseDirectories, BaseDirectoriesError};

use crate::command_line::{
    Action as CommandLineAction, CommandLine, EarlyExitTriggers, Fullscreen,
};
use crate::tools::Tools;

pub static APP_CONFIG: SharedState<Configuration> = SharedState::new();

#[derive(Error, Debug)]
enum ConfigurationFileError {
    #[error("XDG context error: {0}")]
    Xdg(#[from] BaseDirectoriesError),

    #[error("Error reading file: {0}")]
    ReadFile(#[from] io::Error),

    #[error("Decoding toml failed: {0}")]
    TomlDecoding(#[from] toml::de::Error),
}

pub struct Configuration {
    input_filename: String,
    output_filename: Option<String>,
    fullscreen: Option<Fullscreen>,
    early_exit: EarlyExit,
    corner_roundness: f32,
    initial_tool: Tools,
    actions_on_enter: Vec<Action>,
    actions_on_escape: Vec<Action>,
    disable_notifications: bool,
    title: Option<String>,
}

/// Single-key tool shortcuts. Digits are not used.
pub fn tool_for_key(key: char) -> Option<Tools> {
    match key {
        'b' => Some(Tools::Brush),
        'i' => Some(Tools::Line),
        'z' => Some(Tools::Arrow),
        'r' => Some(Tools::Rectangle),
        _ => None,
    }
}

pub fn key_for_tool(tool: Tools) -> Option<char> {
    match tool {
        Tools::Brush => Some('b'),
        Tools::Line => Some('i'),
        Tools::Arrow => Some('z'),
        Tools::Rectangle => Some('r'),
        Tools::Crop => None,
    }
}

// remain compatible with old config with fullscreen=true/false
#[derive(Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "kebab-case")]
enum FullscreenCompat {
    Bool(bool),
    Mode(Fullscreen),
}

fn de_fullscreen_mode<'de, D>(d: D) -> Result<Option<Fullscreen>, D::Error>
where
    D: Deserializer<'de>,
{
    match FullscreenCompat::deserialize(d)? {
        FullscreenCompat::Bool(true) => Ok(Some(Fullscreen::CurrentScreen)),
        FullscreenCompat::Bool(false) => Ok(None),
        FullscreenCompat::Mode(m) => Ok(Some(m)),
    }
}

// backwards compatibility, allow early-exit = true in config
#[derive(Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "kebab-case")]
enum EarlyExitCompat {
    Bool(bool),
    Triggers(Vec<EarlyExitTriggers>),
}

fn de_early_exit<'de, D>(d: D) -> Result<Option<Vec<EarlyExitTriggers>>, D::Error>
where
    D: Deserializer<'de>,
{
    match EarlyExitCompat::deserialize(d)? {
        EarlyExitCompat::Bool(true) => Ok(Some(vec![EarlyExitTriggers::All])),
        EarlyExitCompat::Bool(false) => Ok(None),
        EarlyExitCompat::Triggers(m) => Ok(Some(m)),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EarlyExit {
    copy: bool,
    save: bool,
    save_as: bool,
}

impl From<Vec<EarlyExitTriggers>> for EarlyExit {
    fn from(val: Vec<EarlyExitTriggers>) -> Self {
        let mut ee = EarlyExit::default();
        for e in val {
            match e {
                EarlyExitTriggers::All => {
                    ee.copy = true;
                    ee.save = true;
                    ee.save_as = true;
                }
                EarlyExitTriggers::Copy => ee.copy = true,
                EarlyExitTriggers::Save => ee.save = true,
                EarlyExitTriggers::SaveAs => ee.save_as = true,
            }
        }
        ee
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    SaveToClipboard,
    SaveToFile,
    SaveToFileAs,
    Exit,
}

impl From<CommandLineAction> for Action {
    fn from(action: CommandLineAction) -> Self {
        match action {
            CommandLineAction::SaveToClipboard => Self::SaveToClipboard,
            CommandLineAction::SaveToFile => Self::SaveToFile,
            CommandLineAction::SaveToFileAs => Self::SaveToFileAs,
            CommandLineAction::Exit => Self::Exit,
        }
    }
}

impl Configuration {
    pub fn load() {
        // parse commandline options and exit if error
        let command_line = match CommandLine::try_parse() {
            Ok(cmd) => cmd,
            Err(e) => e.exit(),
        };

        // read configuration file and exit on error
        let file = match ConfigurationFile::try_read(&command_line.config) {
            Ok(c) => c,
            Err(ConfigurationFileError::ReadFile(e)) if e.kind() == io::ErrorKind::NotFound => {
                eprintln!("config file not found");
                None
            }
            Err(e) => {
                eprintln!("Error reading config file: {e}");

                // swallow broken pipes
                let _ = std::io::stdout().lock().flush();
                let _ = std::io::stderr().lock().flush();

                std::process::exit(3);
            }
        };

        APP_CONFIG.write().merge(file, command_line);
    }

    fn merge_general(&mut self, general: ConfigurationFileGeneral) {
        if let Some(v) = general.fullscreen {
            self.fullscreen = Some(v);
        }
        if let Some(v) = general.early_exit {
            self.early_exit = v.into();
        }
        if let Some(v) = general.corner_roundness {
            self.corner_roundness = v;
        }
        if let Some(v) = general.initial_tool {
            self.initial_tool = v;
        }
        if let Some(v) = general.output_filename {
            self.output_filename = Some(v);
        }
        if let Some(v) = general.actions_on_enter {
            self.actions_on_enter = v;
        }
        if let Some(v) = general.actions_on_escape {
            self.actions_on_escape = v;
        }
        if let Some(v) = general.disable_notifications {
            self.disable_notifications = v;
        }
        if let Some(v) = general.title {
            self.title = Some(v);
        }
    }

    fn merge(&mut self, file: Option<ConfigurationFile>, command_line: CommandLine) {
        // input_filename is required and needs to be overwritten
        self.input_filename = command_line.filename.unwrap_or_default();

        // overwrite with all specified values from config file
        if let Some(general) = file.and_then(|file| file.general) {
            self.merge_general(general);
        }

        // overwrite with all specified values from command line
        if let Some(v) = command_line.fullscreen {
            self.fullscreen = Some(v);
        }
        if let Some(v) = command_line.early_exit {
            self.early_exit = v.into();
        }
        if let Some(v) = command_line.corner_roundness {
            self.corner_roundness = v;
        }
        if let Some(v) = command_line.initial_tool {
            self.initial_tool = v.into();
        }
        if let Some(v) = command_line.output_filename {
            self.output_filename = Some(v);
        }
        if let Some(v) = command_line.actions_on_enter {
            self.actions_on_enter = v.into_iter().map(Into::into).collect();
        }
        if let Some(v) = command_line.actions_on_escape {
            self.actions_on_escape = v.into_iter().map(Into::into).collect();
        }
        if command_line.disable_notifications {
            self.disable_notifications = true;
        }
        if let Some(v) = command_line.title {
            self.title = Some(v);
        }
    }

    pub fn early_exit_copy(&self) -> bool {
        self.early_exit.copy
    }

    pub fn early_exit_save(&self) -> bool {
        self.early_exit.save
    }

    pub fn early_exit_save_as(&self) -> bool {
        self.early_exit.save_as
    }

    pub fn corner_roundness(&self) -> f32 {
        self.corner_roundness
    }

    pub fn initial_tool(&self) -> Tools {
        self.initial_tool
    }

    pub fn fullscreen(&self) -> Option<Fullscreen> {
        self.fullscreen
    }

    pub fn output_filename(&self) -> Option<&String> {
        self.output_filename.as_ref()
    }

    pub fn input_filename(&self) -> &str {
        &self.input_filename
    }

    pub fn actions_on_enter(&self) -> Vec<Action> {
        self.actions_on_enter.clone()
    }

    pub fn actions_on_escape(&self) -> Vec<Action> {
        self.actions_on_escape.clone()
    }

    pub fn disable_notifications(&self) -> bool {
        self.disable_notifications
    }

    pub fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            input_filename: String::new(),
            output_filename: None,
            fullscreen: None,
            early_exit: EarlyExit::default(),
            corner_roundness: 12.0,
            initial_tool: Tools::Rectangle,
            actions_on_enter: vec![],
            actions_on_escape: vec![Action::Exit],
            disable_notifications: false,
            title: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfigurationFile {
    general: Option<ConfigurationFileGeneral>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfigurationFileGeneral {
    #[serde(deserialize_with = "de_fullscreen_mode", default)]
    fullscreen: Option<Fullscreen>,
    #[serde(deserialize_with = "de_early_exit", default)]
    early_exit: Option<Vec<EarlyExitTriggers>>,
    corner_roundness: Option<f32>,
    initial_tool: Option<Tools>,
    output_filename: Option<String>,
    actions_on_enter: Option<Vec<Action>>,
    actions_on_escape: Option<Vec<Action>>,
    disable_notifications: Option<bool>,
    title: Option<String>,
}

impl ConfigurationFile {
    fn try_read(
        specified_path: &Option<String>,
    ) -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        match specified_path {
            None => Self::try_read_xdg(),
            Some(p) => Self::try_read_path(p),
        }
    }

    fn try_read_xdg() -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        let dirs = BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"));
        match dirs.get_config_file("config.toml") {
            Some(path) => Self::try_read_path(path),
            None => Ok(None),
        }
    }

    fn try_read_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        let content = fs::read_to_string(path)?;
        Ok(Some(toml::from_str::<ConfigurationFile>(&content)?))
    }
}
