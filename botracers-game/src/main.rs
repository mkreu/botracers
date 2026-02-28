use avian2d::prelude::*;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use botracers_game::{bootstrap, game_api, race_runtime, ui};

fn main() {
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
        use botracers_game::bootstrap;

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
            PhysicsPlugins::default(),
            PhysicsDebugPlugin,
            game_api::GameApiPlugin,
            race_runtime::RaceRuntimePlugin,
            bootstrap::BootstrapPlugin,
            ui::BootstrapUiPlugin,
            ui::RaceRuntimeUiPlugin,
        ))
        .run();
}
