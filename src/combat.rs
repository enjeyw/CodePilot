use bevy::{prelude::*, utils::HashSet, sprite::{collide_aabb::collide, MaterialMesh2dBundle, Mesh2dHandle}, render::mesh};

use crate::{PlayerState, WinSize, EnemyCount, components::{SpriteSize, Laser, FromPlayer, Enemy, FromEnemy, Player, ExplosionToSpawn, Explosion, ExplosionTimer, Weapon, Ship, Shield, EMPAnimator, EMP, Allegiance, WeaponType}, GameTextures, EXPLOSION_LEN, CollidedEntities, events::FireWeaponEvent};
use bevy::prelude::Entity;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_event::<FireWeaponEvent>()
        .add_systems(Update, weapon_cooldown_system)
        .add_systems(Update, laser_hit_system)
        .add_systems(Update, explosion_to_spawn_system)
        .add_systems(Update, explosion_animation_system)
        .add_systems(Update, try_fire_emp_listener)
        .add_systems(Update, ship_destroyed_system)
        .add_systems(Update, ship_shield_charge_system)
        .add_systems(Update, ship_shield_sprite_system)
        .add_systems(Update, emp_animation_system);
    }
}

fn weapon_cooldown_system(
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	win_size: Res<WinSize>,
    mut weapon: Query<(&mut Weapon)>,
) {
	if player_state.weapon_cooldown > 0. {
		player_state.weapon_cooldown -= time.delta_seconds();

		if player_state.weapon_cooldown <= 0. {
			info!("Ready to fire!");
		} 
	}

    for (mut weapon) in weapon.iter_mut() {
        if weapon.current_charge >= 1. {
            continue;
        }
        weapon.current_charge += weapon.charge_rate * time.delta_seconds();
    }


}

fn try_fire_emp_listener(
    mut ev_weapon_fired: EventReader<FireWeaponEvent>,
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    mut weapon_query: Query<(&Parent, &mut Weapon)>,
    mut ship_query: Query<(&mut Ship, &Allegiance, &Transform)>
) {

    for fire_event in ev_weapon_fired.read() {

        // Filter out non-EMP weapons
        if fire_event.weapon_type != WeaponType::EMP {
            continue;
        }

        // Iterate through weapons to find the one that fired
        for (parent, mut fired_weapon) in weapon_query.iter_mut() {
            if parent.get() != fire_event.firing_entity {
                continue;
            }

            if fired_weapon.current_charge < 1. {
                continue;
            }

            let mut firing_xy: Option<(f32,f32)> = None;
            let mut firing_ship_allegiance: Option<Allegiance> = None;


            if let Ok((firing_ship, fsa, firing_ship_tf)) = ship_query.get_mut(fire_event.firing_entity) {
                fired_weapon.current_charge = 0.;
                firing_xy = Some((firing_ship_tf.translation.x, firing_ship_tf.translation.y));
                firing_ship_allegiance = Some(fsa.clone());

            } else {
                continue;
            }

            info!("Firing EMP");


            if let (Some((x, y)), Some(fsa)) = (firing_xy, firing_ship_allegiance) {

                commands.spawn({
                    SpriteBundle {
                        texture: game_textures.emp.clone(),
                        sprite: Sprite {
                            color: Color::rgb(0.0, 0.0, 90.0),
                            ..Default::default()
                        },
                        transform: Transform {
                            translation: Vec3::new(x, y, 0.),
                            scale: Vec3::new(1., 1., 1.),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }).insert(EMPAnimator::new(1.1));

                // deal damage to enemy ships inversely proportional to distance
                for (mut ship, ship_allegiance, ship_tf) in ship_query.iter_mut() {
                    if *ship_allegiance == fsa {
                        continue;
                    }

                    let (x2, y2) = (ship_tf.translation.x, ship_tf.translation.y);
                    let distance = ((x2 - x).powi(2) + (y2 - y).powi(2)).sqrt();

                    let damage = (80. / distance).min(0.2);

                    info!("Damage {} to ship", damage);

                    ship.current_shields -= damage;

                }
                
            }
        }
    }

	
}

#[allow(clippy::type_complexity)] // for the Query types.
fn laser_hit_system(
	mut commands: Commands,
	mut enemy_count: ResMut<EnemyCount>,
	mut player_state: ResMut<PlayerState>,
    mut occured_collisions: ResMut<CollidedEntities>,
	laser_query: Query<(Entity, &Allegiance, &Transform, &SpriteSize), (With<Laser>)>,
	mut ship_query: Query<(Entity, &mut Ship, &Allegiance, &Transform, &SpriteSize)>
) {
	let mut despawned_entities: HashSet<Entity> = HashSet::new();

	// iterate through the lasers
	for (laser_entity, laser_allegiance, laser_tf, laser_size) in laser_query.iter() {
		if despawned_entities.contains(&laser_entity) {
			continue;
		}

		let laser_scale = laser_tf.scale.xy();

		// iterate through ships
		for (ship_entity, mut ship, ship_allegiance, ship_tf, ship_size) in ship_query.iter_mut() {

            if despawned_entities.contains(&ship_entity)
				|| despawned_entities.contains(&laser_entity)
			{
				continue;
			}

            if occured_collisions.0.contains(&(laser_entity, ship_entity)) {
                continue;
            }

            if ship_allegiance == laser_allegiance {
                continue;
            }

			let ship_scale = ship_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				ship_tf.translation,
				ship_size.0 * ship_scale,
			);

			// perform collision
			if collision.is_some() {
				// remove the laser
				// commands.entity(laser_entity).despawn();

                // mark the laser and ship as collided
                occured_collisions.0.insert((laser_entity, ship_entity));

                // add damage
				ship.current_shields -= 0.81;

				break;
			}
		}
	}
}


// spawn a shield sprite
pub fn spawn_shield_sprite(parent: &mut ChildBuilder, game_textures: Res<GameTextures>) {
    parent
        .spawn(SpriteBundle {
            texture: game_textures.shield.clone(),
            sprite: Sprite {
                // Alpha channel of the color controls transparency.
                color: Color::rgba(9.0, 9.0, 9.0, 0.1),
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0., 0., 0.),
                rotation: Quat::from_rotation_z(0.),
                scale: Vec3::new(0.5, 0.5, 1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Shield);
}

// system to show shield charge level on the ship by changing the shield sprite alpha and color
fn ship_shield_sprite_system(
    ship_query: Query<(&Ship, &Children)>,
    mut shield_query: Query<&mut Sprite, With<Shield>>,
) {
    for (ship, children) in ship_query.iter() {
        for child in children.iter() {
            if let Ok(mut sprite) = shield_query.get_mut(child.clone()) {
                let charge_ratio = ship.current_shields / ship.max_shields;
                sprite.color = Color::rgba(9.0 * (1. - charge_ratio.powf(2.0)) + 1.0, 9.0 * (charge_ratio.powf(2.0) - 0.2) + 1.0, 1.0, charge_ratio * 0.2);

                // if charge_ratio >= 1.0 {
                //     sprite.color = Color::rgba(9.0, 9.0, 9.0, 0.2);
                // }

            }
        }
    }

    // for (mut sprite, _shield, ship) in shield_query.iter_mut() {
    //     sprite.color.set_a(ship.current_shields / ship.max_shields);
    // }
}


//system to charge ship shields
fn ship_shield_charge_system(
    mut ship_query: Query<(&mut Ship)>,
    time: Res<Time>,
) {
    for (mut ship) in ship_query.iter_mut() {
        if ship.current_shields >= ship.max_shields {
            continue;
        }

        ship.current_shields += ship.sheild_carge_rate * time.delta_seconds();
    }
}





//system to manage destroyed enemy and player ships
fn ship_destroyed_system(
    mut commands: Commands,
    mut player_state: ResMut<PlayerState>,
    mut enemy_count: ResMut<EnemyCount>,
    time: Res<Time>,
    mut ship_query: Query<(Entity, &mut Ship, &Allegiance, &Transform, Option<&Player>)>,
) {

    let mut despawned_entities: HashSet<Entity> = HashSet::new();

    for (ship_entity, mut ship, allegiance, ship_tf, player) in ship_query.iter_mut() {
       
       if despawned_entities.contains(&ship_entity) {
            continue;
		}

        if ship.current_shields <= 0. {
            commands.entity(ship_entity).despawn_recursive();
            despawned_entities.insert(ship_entity);
      
            commands.spawn((ExplosionToSpawn {
                transform: Transform {
                    translation: ship_tf.translation,
                    ..Default::default()
                },
                duration: 0.05,
                is_engine: false
            },));

            if let Some(player) = player {
                player_state.shot(time.elapsed_seconds_f64());
				player_state.score = 0;
                
            } else {
                enemy_count.0 -= 1;
                player_state.score += 1;
            }
        }
    }
}


fn explosion_to_spawn_system(
	mut commands: Commands,
	game_textures: Res<GameTextures>,
	query: Query<(Entity, &ExplosionToSpawn)>,
) {
	for (explosion_spawn_entity, explosion_to_spawn) in query.iter() {

		let mut sprite_bundle = {
			SpriteSheetBundle {
				texture_atlas: if (explosion_to_spawn.is_engine) {game_textures.engine.clone()} else {game_textures.explosion.clone()},
				transform: explosion_to_spawn.transform.clone(),
				sprite: TextureAtlasSprite {
					color: Color::rgb(5.0, 5.0, 5.0),
					..Default::default()
				}, 
				..Default::default() 
			}
		};

		if (explosion_to_spawn.is_engine) {
			sprite_bundle.sprite.index = 6;
		}

		// spawn the explosion sprite
		commands
			.spawn(sprite_bundle)
			.insert(Explosion)
			.insert(ExplosionTimer::new(explosion_to_spawn.duration));

		// despawn the explosionToSpawn
		commands.entity(explosion_spawn_entity).despawn();
	}
}

fn explosion_animation_system(
	mut commands: Commands,
	time: Res<Time>,
	mut query: Query<(Entity, &mut ExplosionTimer, &mut TextureAtlasSprite), With<Explosion>>,
) {
	for (entity, mut timer, mut sprite) in &mut query {
		timer.0.tick(time.delta());
		if timer.0.finished() {
			sprite.index += 1; // move to next sprite cell
			if sprite.index >= EXPLOSION_LEN {
				commands.entity(entity).despawn();
			}
		}
	}
}

fn emp_animation_system(
	mut commands: Commands,
	time: Res<Time>,
	mut query: Query<(Entity, &mut EMPAnimator, &mut Sprite, &mut Transform)>,
) {
	for (entity, mut animator, mut sprite, mut transform) in &mut query {
        transform.scale.x *= 1.1;
        transform.scale.y *= 1.1;

		animator.timer.tick(time.delta());
		if animator.timer.finished() {
			commands.entity(entity).despawn();
		}
	}
}