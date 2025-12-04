//! Felipe - Cyberpunk File Manager
//!
//! Felipe = anagram of Filepe (FILE + Parallel Experience)
//! Inspired by TRON and Philip's Bookshelf.
//! Orange wireframe aesthetics, vim keybindings, 3D navigation.

use bevy::prelude::*;
use std::path::PathBuf;

// =============================================================================
// Constants - Felipe's Visual Identity
// =============================================================================

/// Felipe Orange - the signature color
const FELIPE_ORANGE: Color = Color::srgb(1.0, 0.4, 0.0);
/// Darker orange for secondary elements
const FELIPE_ORANGE_DIM: Color = Color::srgb(0.6, 0.24, 0.0);
/// Very dim orange for grid
const FELIPE_GRID: Color = Color::srgb(0.3, 0.12, 0.0);
/// Background - pure black for contrast
const FELIPE_BLACK: Color = Color::srgb(0.02, 0.02, 0.02);

/// Spacing between items
const ITEM_SPACING: f32 = 2.0;
/// Base height for files (scaled by size)
const BASE_HEIGHT: f32 = 0.5;
/// Max height for files
const MAX_HEIGHT: f32 = 10.0;
/// Bookshelf depth per GB (reserved for future folder depth visualization)
#[allow(dead_code)]
const DEPTH_PER_GB: f32 = 1.0;

// =============================================================================
// Core State
// =============================================================================

/// Current directory being viewed
#[derive(Resource)]
struct CurrentDirectory {
    path: PathBuf,
    entries: Vec<FileEntry>,
    selected_index: usize,
    needs_reload: bool,
}

impl Default for CurrentDirectory {
    fn default() -> Self {
        Self {
            path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            entries: Vec::new(),
            selected_index: 0,
            needs_reload: true,
        }
    }
}

/// A file or directory entry
#[derive(Clone, Debug)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

/// Vim-like mode
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
enum VimMode {
    #[default]
    Normal,
    Visual,
    /// Reserved for future command mode implementation (e.g., :wq, :q, etc.)
    #[allow(dead_code)]
    Command,
}

/// Camera state
#[derive(Resource)]
struct CameraState {
    target: Vec3,
    distance: f32,
    angle: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 30.0,
            angle: 0.8, // radians, looking down at ~45 degrees
        }
    }
}

// =============================================================================
// Components
// =============================================================================

/// Marker for file/folder 3D entities
#[derive(Component)]
struct FileEntity {
    index: usize,
}

/// Marker for file/folder text labels
#[derive(Component)]
struct FileLabel {
    index: usize,
}

/// Marker for the main 3D camera
#[derive(Component)]
struct MainCamera;

/// Marker for UI elements
#[derive(Component)]
struct UiElement;

/// Marker for path display
#[derive(Component)]
struct PathDisplay;

/// Marker for mode indicator
#[derive(Component)]
struct ModeIndicator;

// =============================================================================
// Setup Systems
// =============================================================================

fn setup_camera(mut commands: Commands, camera_state: Res<CameraState>) {
    // 3D Camera - isometric-ish view
    let camera_pos = calculate_camera_position(&camera_state);

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(camera_pos)
                .looking_at(camera_state.target, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));

    // Ambient light (very dim, cyberpunk style)
    commands.insert_resource(AmbientLight {
        color: FELIPE_ORANGE,
        brightness: 50.0,
    });
}

fn setup_ui(mut commands: Commands) {
    // Background panel for path display
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.9)),
            ..default()
        },
        UiElement,
    ))
    .with_children(|parent| {
        parent.spawn((
            TextBundle {
                text: Text::from_section(
                    "",
                    TextStyle {
                        font_size: 24.0,
                        color: FELIPE_ORANGE,
                        ..default()
                    },
                ),
                ..default()
            },
            PathDisplay,
        ));
    });

    // Mode indicator at bottom left
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                "-- NORMAL --",
                TextStyle {
                    font_size: 18.0,
                    color: FELIPE_ORANGE,
                    ..default()
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        },
        ModeIndicator,
        UiElement,
    ));

    // Help text at bottom right
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                "hjkl:move  l/Enter:open  h:back  g/G:top/bottom  v:visual",
                TextStyle {
                    font_size: 16.0,
                    color: FELIPE_ORANGE_DIM,
                    ..default()
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                ..default()
            },
            ..default()
        },
        UiElement,
    ));
}

// =============================================================================
// Directory Loading
// =============================================================================

fn load_directory(mut current_dir: ResMut<CurrentDirectory>) {
    if !current_dir.needs_reload {
        return;
    }

    let path = current_dir.path.clone();
    let mut entries = Vec::new();

    // Add parent directory entry if not root
    if let Some(parent) = path.parent() {
        if parent != path {
            entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                size: 0,
            });
        }
    }

    // Read directory contents
    if let Ok(read_dir) = std::fs::read_dir(&path) {
        let mut dir_entries: Vec<FileEntry> = read_dir
            .filter_map(|e| e.ok())
            .map(|entry| {
                let metadata = entry.metadata().ok();
                FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path(),
                    is_dir: metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                }
            })
            .collect();

        // Sort: directories first, then alphabetically
        dir_entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        entries.extend(dir_entries);
    }

    current_dir.entries = entries;
    current_dir.selected_index = 0;
    current_dir.needs_reload = false;
}

// =============================================================================
// 3D Visualization
// =============================================================================

fn spawn_file_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_dir: Res<CurrentDirectory>,
    existing_entity_query: Query<Entity, With<FileEntity>>,
    existing_label_query: Query<Entity, With<FileLabel>>,
) {
    // Only spawn if directory was just loaded
    if !existing_entity_query.is_empty() || !existing_label_query.is_empty() || current_dir.entries.is_empty() {
        return;
    }

    // Material for files/folders
    let material_normal = materials.add(StandardMaterial {
        base_color: FELIPE_ORANGE_DIM,
        emissive: LinearRgba::new(0.6, 0.24, 0.0, 1.0),
        unlit: true,
        ..default()
    });

    let material_selected = materials.add(StandardMaterial {
        base_color: FELIPE_ORANGE,
        emissive: LinearRgba::new(1.0, 0.4, 0.0, 1.0),
        unlit: true,
        ..default()
    });

    let material_dir = materials.add(StandardMaterial {
        base_color: FELIPE_GRID,
        emissive: LinearRgba::new(0.3, 0.12, 0.0, 1.0),
        unlit: true,
        ..default()
    });

    // Spawn entities for each file/folder
    for (i, entry) in current_dir.entries.iter().enumerate() {
        let x = (i % 10) as f32 * ITEM_SPACING - 9.0;
        let z = (i / 10) as f32 * ITEM_SPACING;

        let height = if entry.is_dir {
            BASE_HEIGHT
        } else {
            // Scale height by file size (log scale)
            let size_mb = entry.size as f32 / (1024.0 * 1024.0);
            (BASE_HEIGHT + size_mb.log10().max(0.0) * 2.0).min(MAX_HEIGHT)
        };

        let mesh = meshes.add(Cuboid::new(0.8, height, 0.3));

        let material = if i == current_dir.selected_index {
            material_selected.clone()
        } else if entry.is_dir {
            material_dir.clone()
        } else {
            material_normal.clone()
        };

        commands.spawn((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_xyz(x, height / 2.0, z),
                ..default()
            },
            FileEntity { index: i },
        ));

        // Spawn text label above the file/folder
        let label_color = if i == current_dir.selected_index {
            FELIPE_ORANGE
        } else {
            FELIPE_ORANGE_DIM
        };

        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    &entry.name,
                    TextStyle {
                        font_size: 30.0,
                        color: label_color,
                        ..default()
                    },
                ),
                transform: Transform::from_xyz(x, height + 1.5, z)
                    .with_scale(Vec3::splat(0.03)),
                ..default()
            },
            FileLabel { index: i },
        ));
    }
}

fn despawn_file_entities(
    mut commands: Commands,
    current_dir: Res<CurrentDirectory>,
    entity_query: Query<Entity, With<FileEntity>>,
    label_query: Query<Entity, With<FileLabel>>,
) {
    if current_dir.needs_reload {
        // Despawn 3D entities
        for entity in entity_query.iter() {
            commands.entity(entity).despawn();
        }
        // Despawn text labels
        for entity in label_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

// =============================================================================
// Grid Drawing
// =============================================================================

fn draw_grid(mut gizmos: Gizmos) {
    let grid_size: i32 = 50;
    let grid_spacing = 2.0;

    // Draw grid lines
    for i in -grid_size..=grid_size {
        let pos = i as f32 * grid_spacing;
        let alpha = 1.0 - (i.abs() as f32 / grid_size as f32) * 0.8;
        let color = FELIPE_GRID.with_alpha(alpha * 0.5);

        // X-axis lines
        gizmos.line(
            Vec3::new(-grid_size as f32 * grid_spacing, 0.0, pos),
            Vec3::new(grid_size as f32 * grid_spacing, 0.0, pos),
            color,
        );
        // Z-axis lines
        gizmos.line(
            Vec3::new(pos, 0.0, -grid_size as f32 * grid_spacing),
            Vec3::new(pos, 0.0, grid_size as f32 * grid_spacing),
            color,
        );
    }
}

// =============================================================================
// Input Handling
// =============================================================================

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_dir: ResMut<CurrentDirectory>,
    mut vim_mode: ResMut<VimMode>,
    mut camera_state: ResMut<CameraState>,
) {
    let entry_count = current_dir.entries.len();
    if entry_count == 0 {
        return;
    }

    match *vim_mode {
        VimMode::Normal => {
            // j or Down - next item
            if keyboard.just_pressed(KeyCode::KeyJ) || keyboard.just_pressed(KeyCode::ArrowDown) {
                current_dir.selected_index = (current_dir.selected_index + 1).min(entry_count - 1);
                update_camera_target(&current_dir, &mut camera_state);
            }
            // k or Up - previous item
            if keyboard.just_pressed(KeyCode::KeyK) || keyboard.just_pressed(KeyCode::ArrowUp) {
                current_dir.selected_index = current_dir.selected_index.saturating_sub(1);
                update_camera_target(&current_dir, &mut camera_state);
            }
            // l or Right or Enter - enter directory / open file
            if keyboard.just_pressed(KeyCode::KeyL)
                || keyboard.just_pressed(KeyCode::ArrowRight)
                || keyboard.just_pressed(KeyCode::Enter)
            {
                if let Some(entry) = current_dir.entries.get(current_dir.selected_index) {
                    if entry.is_dir {
                        current_dir.path = entry.path.clone();
                        current_dir.needs_reload = true;
                    }
                }
            }
            // h or Left - go to parent
            if keyboard.just_pressed(KeyCode::KeyH) || keyboard.just_pressed(KeyCode::ArrowLeft) {
                if let Some(parent) = current_dir.path.parent() {
                    if parent != current_dir.path {
                        current_dir.path = parent.to_path_buf();
                        current_dir.needs_reload = true;
                    }
                }
            }
            // g - go to top
            if keyboard.just_pressed(KeyCode::KeyG) && !keyboard.pressed(KeyCode::ShiftLeft) {
                current_dir.selected_index = 0;
                update_camera_target(&current_dir, &mut camera_state);
            }
            // G (shift+g) - go to bottom
            if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyG) {
                current_dir.selected_index = entry_count - 1;
                update_camera_target(&current_dir, &mut camera_state);
            }
            // v - visual mode
            if keyboard.just_pressed(KeyCode::KeyV) {
                *vim_mode = VimMode::Visual;
            }
        }
        VimMode::Visual | VimMode::Command => {
            if keyboard.just_pressed(KeyCode::Escape) {
                *vim_mode = VimMode::Normal;
            }
        }
    }
}

fn handle_mouse_wheel(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut camera_state: ResMut<CameraState>,
) {
    for event in scroll_events.read() {
        camera_state.distance = (camera_state.distance - event.y * 2.0).clamp(10.0, 100.0);
    }
}

fn update_camera_target(current_dir: &CurrentDirectory, camera_state: &mut CameraState) {
    let i = current_dir.selected_index;
    let x = (i % 10) as f32 * ITEM_SPACING - 9.0;
    let z = (i / 10) as f32 * ITEM_SPACING;
    camera_state.target = Vec3::new(x, 0.0, z);
}

fn calculate_camera_position(camera_state: &CameraState) -> Vec3 {
    let offset = Vec3::new(
        0.0,
        camera_state.distance * camera_state.angle.sin(),
        -camera_state.distance * camera_state.angle.cos(),
    );
    camera_state.target + offset
}

// =============================================================================
// Update Systems
// =============================================================================

fn update_camera(
    camera_state: Res<CameraState>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    for mut transform in camera_query.iter_mut() {
        let target_pos = calculate_camera_position(&camera_state);
        // Smooth interpolation
        transform.translation = transform.translation.lerp(target_pos, 0.1);
        transform.look_at(camera_state.target, Vec3::Y);
    }
}

fn update_file_materials(
    current_dir: Res<CurrentDirectory>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&FileEntity, &Handle<StandardMaterial>)>,
) {
    for (file_entity, material_handle) in query.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            let is_selected = file_entity.index == current_dir.selected_index;
            let entry = current_dir.entries.get(file_entity.index);
            let is_dir = entry.map(|e| e.is_dir).unwrap_or(false);

            if is_selected {
                material.base_color = FELIPE_ORANGE;
                material.emissive = LinearRgba::new(1.0, 0.4, 0.0, 1.0);
            } else if is_dir {
                material.base_color = FELIPE_GRID;
                material.emissive = LinearRgba::new(0.3, 0.12, 0.0, 1.0);
            } else {
                material.base_color = FELIPE_ORANGE_DIM;
                material.emissive = LinearRgba::new(0.6, 0.24, 0.0, 1.0);
            }
        }
    }
}

fn update_file_labels(
    current_dir: Res<CurrentDirectory>,
    mut label_query: Query<(&FileLabel, &mut Text)>,
) {
    for (file_label, mut text) in label_query.iter_mut() {
        let is_selected = file_label.index == current_dir.selected_index;
        text.sections[0].style.color = if is_selected {
            FELIPE_ORANGE
        } else {
            FELIPE_ORANGE_DIM
        };
    }
}

fn update_ui(
    current_dir: Res<CurrentDirectory>,
    vim_mode: Res<VimMode>,
    mut path_query: Query<&mut Text, With<PathDisplay>>,
    mut mode_query: Query<&mut Text, (With<ModeIndicator>, Without<PathDisplay>)>,
) {
    // Update path display
    for mut text in path_query.iter_mut() {
        let selected_entry = current_dir.entries.get(current_dir.selected_index);
        let selected_name = selected_entry.map(|e| e.name.as_str()).unwrap_or("");
        let file_info = if let Some(entry) = selected_entry {
            if entry.is_dir {
                format!(" [DIR]")
            } else {
                format!(" [{:.2} MB]", entry.size as f64 / (1024.0 * 1024.0))
            }
        } else {
            String::new()
        };

        text.sections[0].value = format!(
            "📂 {}\n▶ {}{}",
            current_dir.path.to_string_lossy(),
            selected_name,
            file_info
        );
    }

    // Update mode indicator
    for mut text in mode_query.iter_mut() {
        text.sections[0].value = match *vim_mode {
            VimMode::Normal => "-- NORMAL --".to_string(),
            VimMode::Visual => "-- VISUAL --".to_string(),
            VimMode::Command => ":".to_string(),
        };
    }
}

// =============================================================================
// App Entry Point
// =============================================================================

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Felipe - File Manager".to_string(),
                resolution: (1200., 800.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELIPE_BLACK))
        .insert_resource(CurrentDirectory::default())
        .insert_resource(VimMode::default())
        .insert_resource(CameraState::default())
        .add_systems(Startup, (setup_camera, setup_ui))
        .add_systems(
            Update,
            (
                load_directory,
                despawn_file_entities,
                spawn_file_entities,
                handle_keyboard,
                handle_mouse_wheel,
                update_camera,
                update_file_materials,
                update_file_labels,
                update_ui,
                draw_grid,
            ),
        )
        .run();
}
