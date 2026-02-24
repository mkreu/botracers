use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverType {
    RemoteArtifact { id: i64 },
}

impl DriverType {
    pub fn label(&self) -> String {
        match self {
            DriverType::RemoteArtifact { id } => format!("Artifact: #{id}"),
        }
    }
}

#[derive(Message)]
pub struct SpawnCarRequest {
    pub driver: DriverType,
}

#[derive(Message)]
pub struct SpawnResolvedCarRequest {
    pub driver: DriverType,
    pub elf_bytes: Vec<u8>,
    #[allow(dead_code)]
    pub binary_name: String,
}

#[derive(Message)]
pub enum WebApiCommand {
    RefreshCapabilities,
    LoadArtifacts,
    UploadArtifact,
    DeleteArtifact { id: i64 },
    SetArtifactVisibility { id: i64, is_public: bool },
}

pub struct GameApiPlugin;

impl Plugin for GameApiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnCarRequest>()
            .add_message::<SpawnResolvedCarRequest>()
            .add_message::<WebApiCommand>();
    }
}
