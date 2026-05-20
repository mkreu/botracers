use bevy::prelude::*;
use botracers_race_runtime::ArtifactId;


#[derive(Message)]
pub struct SpawnCarRequest(pub ArtifactId);

#[derive(Message)]
pub enum WebApiCommand {
    RefreshCapabilities,
    LoadArtifacts,
    UploadArtifact,
    DeleteArtifact { id: ArtifactId },
    SetArtifactVisibility { id: ArtifactId, is_public: bool },
}

pub struct GameApiPlugin;

impl Plugin for GameApiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnCarRequest>()
            .add_message::<WebApiCommand>();
    }
}
