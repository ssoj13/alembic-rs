//! Persistent application settings

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings that persist between sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Display
    pub show_grid: bool,
    pub show_wireframe: bool,
    pub flat_shading: bool,
    pub show_shadows: bool,
    pub xray_alpha: f32,
    pub double_sided: bool,
    pub auto_normals: bool,
    pub smooth_normals: bool,
    pub smooth_angle: f32,  // 0-180 degrees
    pub show_floor: bool,
    pub background_color: [f32; 4],
    
    // Window
    pub window_width: f32,
    pub window_height: f32,
    
    // Camera
    pub camera_distance: f32,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    
    // Last opened file
    pub last_file: Option<PathBuf>,
    
    // Recent files (most recent first, max 10)
    pub recent_files: Vec<PathBuf>,
    
    // Environment
    pub hdr_enabled: bool,
    pub hdr_visible: bool,
    pub hdr_exposure: f32,
    pub last_hdr_file: Option<PathBuf>,
    
    // Anti-aliasing (requires restart)
    pub antialiasing: u8,
    
    // UI layout
    pub hierarchy_panel_width: f32,
    pub side_panel_width: f32,
    
    // Playback
    pub playback_fps: f32,
    
    // Lighting
    pub use_scene_lights: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_wireframe: false,
            flat_shading: false,
            show_shadows: true,
            xray_alpha: 1.0,
            double_sided: false,
            auto_normals: true,  // Auto-flip backface normals by default
            smooth_normals: false,
            smooth_angle: 45.0,  // Default 45 degrees
            show_floor: false,
            background_color: [0.1, 0.1, 0.12, 1.0],
            window_width: 1280.0,
            window_height: 720.0,
            camera_distance: 5.0,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            last_file: None,
            recent_files: Vec::new(),
            hdr_enabled: false,
            hdr_visible: true,
            hdr_exposure: 1.0,
            last_hdr_file: None,
            antialiasing: 4,
            hierarchy_panel_width: 200.0,
            side_panel_width: 200.0,
            playback_fps: 24.0,
            use_scene_lights: false,
        }
    }
}

const MAX_RECENT_FILES: usize = 10;

impl Settings {
    /// Get settings file path
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("alembic-viewer");
            std::fs::create_dir_all(&p).ok();
            p.push("settings.json");
            p
        })
    }

    /// Load settings from file
    pub fn load() -> Self {
        let mut settings: Self = Self::path()
            .and_then(|p| std::fs::read_to_string(&p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        
        // Validate antialiasing - with TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES we support 1,2,4,8
        if !matches!(settings.antialiasing, 1 | 2 | 4 | 8) {
            settings.antialiasing = 4;
        }
        
        settings
    }

    /// Save settings to file
    pub fn save(&self) {
        if let Some(path) = Self::path() {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
    
    /// Add file to recent files list (moves to top if already present)
    pub fn add_recent(&mut self, path: PathBuf) {
        // Remove if already in list
        self.recent_files.retain(|p| p != &path);
        
        // Insert at front
        self.recent_files.insert(0, path.clone());
        
        // Trim to max size
        self.recent_files.truncate(MAX_RECENT_FILES);
        
        // Also update last_file
        self.last_file = Some(path);
    }
    
    /// Get recent files (filters out non-existent)
    pub fn recent_files(&self) -> Vec<&PathBuf> {
        self.recent_files.iter().filter(|p| p.exists()).collect()
    }
}
