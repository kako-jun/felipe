//! Filepe - Cyberpunk File Manager
//!
//! FILE + Parallel Experience
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
/// Background - pure black for contrast
const FELIPE_BLACK: Color = Color::srgb(0.0, 0.0, 0.0);

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
}

/// Vim-like mode
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
enum VimMode {
    #[default]
    Normal,
    Visual,
    Command,
}

// =============================================================================
// Components for 2D Minimap View
// =============================================================================

/// Marker for file entry display elements
#[derive(Component)]
struct FileEntryDisplay {
    index: usize,
}

/// Marker for the selection cursor
#[derive(Component)]
struct SelectionCursor;

/// Marker for directory path display
#[derive(Component)]
struct PathDisplay;

/// Marker for mode indicator
#[derive(Component)]
struct ModeIndicator;

// =============================================================================
// Systems
// =============================================================================

fn setup(mut commands: Commands) {
    // 2D Camera
    commands.spawn(Camera2dBundle::default());
}

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

fn spawn_ui(
    mut commands: Commands,
    current_dir: Res<CurrentDirectory>,
    entry_query: Query<&FileEntryDisplay>,
) {
    // Don't spawn if UI already exists
    if !entry_query.is_empty() {
        return;
    }

    // Path display at top
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                current_dir.path.to_string_lossy().to_string(),
                TextStyle {
                    font_size: 20.0,
                    color: FELIPE_ORANGE,
                    ..default()
                },
            ),
            transform: Transform::from_xyz(0.0, 320.0, 0.0),
            ..default()
        },
        PathDisplay,
    ));

    // Mode indicator at bottom left
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "-- NORMAL --",
                TextStyle {
                    font_size: 16.0,
                    color: FELIPE_ORANGE_DIM,
                    ..default()
                },
            ),
            transform: Transform::from_xyz(-350.0, -320.0, 0.0),
            ..default()
        },
        ModeIndicator,
    ));

    // File entries
    let start_y = 280.0;
    let line_height = 24.0;
    let max_visible = 20;

    for (i, entry) in current_dir.entries.iter().take(max_visible).enumerate() {
        let prefix = if entry.is_dir { "/" } else { " " };
        let display_name = format!("{}{}", prefix, entry.name);

        let y = start_y - (i as f32 * line_height);
        let color = if i == current_dir.selected_index {
            FELIPE_ORANGE
        } else {
            FELIPE_ORANGE_DIM
        };

        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    display_name,
                    TextStyle {
                        font_size: 18.0,
                        color,
                        ..default()
                    },
                ),
                transform: Transform::from_xyz(-300.0, y, 0.0),
                ..default()
            },
            FileEntryDisplay { index: i },
        ));
    }

    // Selection cursor (wireframe bracket style)
    let cursor_y = start_y - (current_dir.selected_index as f32 * line_height);
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                ">",
                TextStyle {
                    font_size: 18.0,
                    color: FELIPE_ORANGE,
                    ..default()
                },
            ),
            transform: Transform::from_xyz(-330.0, cursor_y, 0.0),
            ..default()
        },
        SelectionCursor,
    ));
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_dir: ResMut<CurrentDirectory>,
    mut vim_mode: ResMut<VimMode>,
) {
    let entry_count = current_dir.entries.len();
    if entry_count == 0 {
        return;
    }

    match *vim_mode {
        VimMode::Normal => {
            // j - down
            if keyboard.just_pressed(KeyCode::KeyJ) {
                current_dir.selected_index = (current_dir.selected_index + 1).min(entry_count - 1);
            }
            // k - up
            if keyboard.just_pressed(KeyCode::KeyK) {
                current_dir.selected_index = current_dir.selected_index.saturating_sub(1);
            }
            // l or Enter - enter directory / open file
            if keyboard.just_pressed(KeyCode::KeyL) || keyboard.just_pressed(KeyCode::Enter) {
                if let Some(entry) = current_dir.entries.get(current_dir.selected_index) {
                    if entry.is_dir {
                        current_dir.path = entry.path.clone();
                        current_dir.needs_reload = true;
                    }
                }
            }
            // h - go to parent
            if keyboard.just_pressed(KeyCode::KeyH) {
                if let Some(parent) = current_dir.path.parent() {
                    if parent != current_dir.path {
                        current_dir.path = parent.to_path_buf();
                        current_dir.needs_reload = true;
                    }
                }
            }
            // g - go to top (gg in real vim, simplified here)
            if keyboard.just_pressed(KeyCode::KeyG) && !keyboard.pressed(KeyCode::ShiftLeft) {
                current_dir.selected_index = 0;
            }
            // G (shift+g) - go to bottom
            if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyG) {
                current_dir.selected_index = entry_count - 1;
            }
            // v - visual mode
            if keyboard.just_pressed(KeyCode::KeyV) {
                *vim_mode = VimMode::Visual;
            }
            // : - command mode
            if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::Semicolon) {
                *vim_mode = VimMode::Command;
            }
        }
        VimMode::Visual | VimMode::Command => {
            // Escape - back to normal
            if keyboard.just_pressed(KeyCode::Escape) {
                *vim_mode = VimMode::Normal;
            }
        }
    }
}

fn update_display(
    current_dir: Res<CurrentDirectory>,
    vim_mode: Res<VimMode>,
    mut cursor_query: Query<&mut Transform, With<SelectionCursor>>,
    mut entry_query: Query<(&FileEntryDisplay, &mut Text)>,
    mut mode_query: Query<&mut Text, (With<ModeIndicator>, Without<FileEntryDisplay>)>,
    mut path_query: Query<
        &mut Text,
        (
            With<PathDisplay>,
            Without<ModeIndicator>,
            Without<FileEntryDisplay>,
        ),
    >,
) {
    let start_y = 280.0;
    let line_height = 24.0;

    // Update cursor position
    for mut transform in cursor_query.iter_mut() {
        transform.translation.y = start_y - (current_dir.selected_index as f32 * line_height);
    }

    // Update entry colors
    for (display, mut text) in entry_query.iter_mut() {
        if let Some(section) = text.sections.first_mut() {
            section.style.color = if display.index == current_dir.selected_index {
                FELIPE_ORANGE
            } else {
                FELIPE_ORANGE_DIM
            };
        }
    }

    // Update mode indicator
    for mut text in mode_query.iter_mut() {
        if let Some(section) = text.sections.first_mut() {
            section.value = match *vim_mode {
                VimMode::Normal => "-- NORMAL --".to_string(),
                VimMode::Visual => "-- VISUAL --".to_string(),
                VimMode::Command => ":".to_string(),
            };
        }
    }

    // Update path display
    for mut text in path_query.iter_mut() {
        if let Some(section) = text.sections.first_mut() {
            section.value = current_dir.path.to_string_lossy().to_string();
        }
    }
}

fn reload_ui_on_change(
    current_dir: Res<CurrentDirectory>,
    mut commands: Commands,
    entry_query: Query<Entity, With<FileEntryDisplay>>,
    cursor_query: Query<Entity, With<SelectionCursor>>,
    path_query: Query<Entity, With<PathDisplay>>,
    mode_query: Query<Entity, With<ModeIndicator>>,
) {
    // If directory needs reload, despawn old UI
    if current_dir.needs_reload {
        for entity in entry_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in cursor_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in path_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in mode_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

// =============================================================================
// App Entry Point
// =============================================================================

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Filepe - File Manager".to_string(),
                resolution: (800., 700.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELIPE_BLACK))
        .insert_resource(CurrentDirectory::default())
        .insert_resource(VimMode::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                reload_ui_on_change,
                load_directory,
                spawn_ui,
                update_display,
            )
                .chain(),
        )
        .run();
}
