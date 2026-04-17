//! UI模块
//!
//! 使用gpui构建用户界面

mod authorization;
mod body;
mod components;
mod headers;
mod json_editor;
mod main_view;
mod scripts;
mod settings;
mod themes;

pub use authorization::*;
pub use body::*;
pub use components::*;
pub use headers::*;
pub use json_editor::*;
pub use main_view::*;
pub use scripts::*;
pub use settings::*;
pub use themes::*;

use crate::app::AppState;
use gpui::*;
use gpui_component::StyledExt;
