use bevy::ecs::{event::Event, entity::Entity};

use crate::components::{WeaponType, Allegiance};

#[derive(Event)]
pub struct FireWeaponEvent {
    pub weapon_type: WeaponType,
    pub weapon_alignment: Allegiance,
    pub firing_entity: Entity
}

#[derive(Event)]
pub struct CompileCodeEvent;

