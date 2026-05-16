use bevy::{
    color::palettes::css::WHITE,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Startup, set_default_zoom.after(setup))
            .add_systems(Update, update_fps_counter)
            .add_systems(Update, update_camera);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        Text::new("FPS: --"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(WHITE.into()),
        FpsCounterText,
    ));

    commands.spawn(Camera2d);
}

#[derive(Component)]
struct FpsCounterText;

fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsCounterText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };

    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|value| value.smoothed())
    {
        text.0 = format!("FPS: {fps:>3.0}");
    }
}

fn set_default_zoom(mut camera_query: Query<&mut Projection, With<Camera2d>>) {
    let Ok(mut projection) = camera_query.single_mut() else {
        return;
    };

    if let Projection::Orthographic(ref mut ortho) = *projection {
        ortho.scale = 0.05;
    }
}

// TODO IMPLEMENT ATTACHING THIS
#[derive(Component)]
struct CameraFollow;

fn update_camera(
    follow_query: Query<&Transform, With<CameraFollow>>,
    mut camera_query: Query<
        (&mut Transform, &mut Projection),
        (With<Camera2d>, Without<CameraFollow>),
    >,
    mut scroll_events: MessageReader<MouseWheel>,
    mut motion_events: MessageReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    let Ok((mut camera_transform, mut projection)) = camera_query.single_mut() else {
        return;
    };

    let mut current_scale = 0.05_f32;
    if let Projection::Orthographic(ref mut ortho) = *projection {
        for event in scroll_events.read() {
            let zoom_delta = match event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => event.y * 0.1,
                bevy::input::mouse::MouseScrollUnit::Pixel => event.y * 0.001,
            };

            ortho.scale *= 1.0 - zoom_delta;
            ortho.scale = ortho.scale.clamp(0.001, 10.0);
        }
        current_scale = ortho.scale;
    }

    if let Ok(follow_transform) = follow_query.single() {
        camera_transform.translation.x = follow_transform.translation.x;
        camera_transform.translation.y = follow_transform.translation.y;
        return;
    }

    if mouse_buttons.pressed(MouseButton::Middle) || mouse_buttons.pressed(MouseButton::Right) {
        for event in motion_events.read() {
            camera_transform.translation.x -= event.delta.x * current_scale;
            camera_transform.translation.y += event.delta.y * current_scale;
        }
    }
}
