use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutStyle {
    Tiles,
    List,
    Centred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabVisibility {
    Always,
    LandscapeOnly,
    PortraitOnly,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Typeface {
    Normal,
    Sans,
    Serif,
    Monospace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelColour {
    White,
    Black,
    Grey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Auto,
    Landscape,
    Portrait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub grid_columns_portrait: u32,
    pub grid_columns_landscape: u32,
    pub layout: LayoutStyle,
    pub tab_visibility: TabVisibility,
    pub wallpaper_alpha: u8,
    pub status_bar_visible: bool,
    pub icons_visible: bool,
    pub icon_size: u32,
    pub icon_alpha: u8,
    pub labels_visible: bool,
    pub label_size: u32,
    pub label_typeface: Typeface,
    pub label_colour: LabelColour,
    pub label_shadow: bool,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsPatch {
    pub grid_columns_portrait: Option<u32>,
    pub grid_columns_landscape: Option<u32>,
    pub layout: Option<LayoutStyle>,
    pub tab_visibility: Option<TabVisibility>,
    pub wallpaper_alpha: Option<u8>,
    pub status_bar_visible: Option<bool>,
    pub icons_visible: Option<bool>,
    pub icon_size: Option<u32>,
    pub icon_alpha: Option<u8>,
    pub labels_visible: Option<bool>,
    pub label_size: Option<u32>,
    pub label_typeface: Option<Typeface>,
    pub label_colour: Option<LabelColour>,
    pub label_shadow: Option<bool>,
    pub orientation: Option<Orientation>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            grid_columns_portrait: 4,
            grid_columns_landscape: 6,
            layout: LayoutStyle::Tiles,
            tab_visibility: TabVisibility::Always,
            wallpaper_alpha: 0,
            status_bar_visible: true,
            icons_visible: true,
            icon_size: 40,
            icon_alpha: 100,
            labels_visible: true,
            label_size: 14,
            label_typeface: Typeface::Normal,
            label_colour: LabelColour::White,
            label_shadow: true,
            orientation: Orientation::Auto,
        }
    }
}

impl Settings {
    /// Pulls every ranged field into its permitted range, keeping the
    /// nearest bound rather than resetting to the default.
    pub fn clamp(&mut self) {
        self.grid_columns_portrait = self.grid_columns_portrait.clamp(1, 12);
        self.grid_columns_landscape = self.grid_columns_landscape.clamp(1, 12);
        self.wallpaper_alpha = self.wallpaper_alpha.min(100);
        self.icon_size = self.icon_size.min(100);
        self.icon_alpha = self.icon_alpha.min(100);
        self.label_size = self.label_size.min(100);
    }

    /// Overwrites the fields the patch sets. Does not clamp; call
    /// [`Settings::clamp`] afterwards.
    pub fn apply(&mut self, patch: &SettingsPatch) {
        macro_rules! overwrite {
            ($($field:ident),* $(,)?) => {
                $(if let Some(v) = patch.$field { self.$field = v; })*
            };
        }
        overwrite!(
            grid_columns_portrait,
            grid_columns_landscape,
            layout,
            tab_visibility,
            wallpaper_alpha,
            status_bar_visible,
            icons_visible,
            icon_size,
            icon_alpha,
            labels_visible,
            label_size,
            label_typeface,
            label_colour,
            label_shadow,
            orientation,
        );
    }
}
