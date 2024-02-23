use bevy::prelude::*;
use bevy_xpbd_3d::math::Quaternion;
use serde::{Deserialize, Serialize};

pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Food>()
            .add_systems(Update, rotate_food);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Component, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize)]
pub struct Food;

fn rotate_food(
    mut query: Query<&mut Transform, With<Food>>
) {
    for mut transform in query.iter_mut() {
        transform.rotate_y(f32::to_radians(10.0));
    }
}