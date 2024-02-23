use bevy::prelude::*;
use bevy_xpbd_3d::math::Quaternion;
use bevy_xpbd_3d::prelude::*;
use serde::{Deserialize, Serialize};
use crate::level_instantiation::spawning::objects::orb::Orb;
use crate::player_control::player_embodiment::Player;

pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Food>()
            .add_systems(Update, (spawn, rotate_food, remove_food));
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

fn spawn(
    query: Query<Entity, Added<Food>>,
    mut commands: Commands
) {
    for entity in query.iter() {
        commands.entity(entity)
            .insert((
                Collider::ball(1.0),
                Sensor
            ));
    }
}

fn remove_food(
    mut collision_event_reader: EventReader<Collision>,
    food: Query<Entity, With<Food>>,
    player_query: Query<Entity, With<Player>>,
    mut commands: Commands
) {
    let Ok(mut player) = player_query.get_single() else {
        // no player, just skip
        return;
    };

    for Collision(contacts) in collision_event_reader.read() {
        if player != contacts.entity1 && player != contacts.entity2 {
            continue; // Skip the loop if the player is not involved in the collision
        }

        if let Ok(food) = food.get(contacts.entity1) {
            commands.entity(food).despawn_recursive();
        }
        if let Ok(food) = food.get(contacts.entity2) {
            commands.entity(food).despawn_recursive();
        }
    }
}