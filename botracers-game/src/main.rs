use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use botracers_race_runtime::{RaceRuntimePlugin, Track};

use botracers_game::{camera::CameraPlugin, track_format::TrackFile};

//mod bootstrap;
//mod game_api;
//mod race_runtime;
//mod ui;

fn main() {
    let track_file = TrackFile::load_builtin().expect("Failed to load built-in track");

    let track = Track::new(track_file
            .control_points
            .iter()
            .map(|&[x, y]| Vec2::new(x, y))
            .collect(),
        track_file.metadata.track_width,
        track_file
            .barriers
            .iter()
            .map(|b| {
                b.points
                    .iter()
                    .map(|&[x, y]| Vec2::new(x, y))
                    .collect::<Vec<_>>()
            })
            .collect());

    App::new()
        .insert_resource(track)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "BotRacers".into(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            // PhysicsPlugins::default(),
            //game_api::GameApiPlugin,
            //race_runtime::RaceRuntimePlugin,
            RaceRuntimePlugin,
            CameraPlugin,
            //bootstrap::BootstrapPlugin,
            //ui::BootstrapUiPlugin,
            //ui::RaceRuntimeUiPlugin,
        ))
        .run();
}

/*fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let mut standalone_mode = false;
    for arg in std::env::args().skip(1) {
        #[cfg(not(target_arch = "wasm32"))]
        if arg == "--standalone" {
            standalone_mode = true;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let bootstrap_config = if standalone_mode {
        let bind = std::env::var("BOTRACERS_STANDALONE_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8787".to_string());
        bootstrap::BootstrapConfig {
            standalone_mode: true,
            standalone_bind: Some(bind),
        }
    } else {
        bootstrap::BootstrapConfig::default()
    };

    #[cfg(target_arch = "wasm32")]
    let bootstrap_config = bootstrap::BootstrapConfig::default();

    App::new()
        .insert_resource(bootstrap_config)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "BotRacers".into(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            // PhysicsPlugins::default(),
            //game_api::GameApiPlugin,
            //race_runtime::RaceRuntimePlugin,
            RaceRuntimePlugin,
            bootstrap::BootstrapPlugin,
            //ui::BootstrapUiPlugin,
            //ui::RaceRuntimeUiPlugin,
        ))
        .run();
}*/
