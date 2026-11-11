use clap::{Parser, ValueEnum};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, name = "rocks")]
pub struct CommandLine {
    /// Path to the config file. Otherwise will be read from XDG_CONFIG_DIR/rocks/config.toml
    #[arg(short, long)]
    pub config: Option<String>,

    /// Path to the input PNG image
    #[arg(short, long, required = true)]
    pub filename: Option<String>,

    /// Start rocks in fullscreen mode.
    /// --fullscreen without parameter is equivalent to --fullscreen current-screen.
    #[arg(long, num_args = 0..=1, default_missing_value = "current-screen", value_enum)]
    pub fullscreen: Option<Fullscreen>,

    /// Filename to use for the save action. Omit to disable saving to file. Might contain format
    /// specifiers: <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>.
    /// Can start with a tilde (~) for the home directory.
    #[arg(short, long)]
    pub output_filename: Option<String>,

    /// Exit directly after the given actions
    #[arg(long, value_delimiter = ',', num_args=0.., default_missing_value = "all")]
    pub early_exit: Option<Vec<EarlyExitTriggers>>,

    /// Draw corners of rectangles round if the value is greater than 0
    /// (Defaults to 12) (0 disables rounded corners)
    #[arg(long)]
    pub corner_roundness: Option<f32>,

    /// Select the tool on startup
    #[arg(long, value_name = "TOOL", visible_alias = "init-tool")]
    pub initial_tool: Option<Tools>,

    /// Actions to perform when pressing Enter
    #[arg(long, value_delimiter = ',')]
    pub actions_on_enter: Option<Vec<Action>>,

    /// Actions to perform when pressing Escape
    #[arg(long, value_delimiter = ',')]
    pub actions_on_escape: Option<Vec<Action>>,

    /// Disable notifications
    #[arg(long)]
    pub disable_notifications: bool,

    /// Set the window title
    #[arg(long)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, ValueEnum, PartialEq)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum Fullscreen {
    All,
    CurrentScreen,
}

#[derive(Debug, Deserialize, Clone, Copy, ValueEnum, PartialEq)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum EarlyExitTriggers {
    All,
    Copy,
    Save,
    SaveAs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Tools {
    Line,
    Arrow,
    Rectangle,
    Brush,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Action {
    SaveToClipboard,
    SaveToFile,
    SaveToFileAs,
    Exit,
}
