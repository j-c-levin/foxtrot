use bevy::prelude::*;
use bevy_water::{WaterPlugin, WaterSettings};

pub struct WaterBasedPlugin;

const WATER_HEIGHT: f32 = -20.0;

impl Plugin for WaterBasedPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WaterSettings {
            height: WATER_HEIGHT,
            update_materials: true,
            ..default()
        })
            .add_plugins(WaterPlugin);
    }
}