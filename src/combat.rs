use bevy::{prelude::*, utils::HashSet, sprite::collide_aabb::collide};

use crate::{PlayerState, WinSize, EnemyCount, components::{SpriteSize, Laser, FromPlayer, Enemy, FromEnemy, Player, ExplosionToSpawn, Explosion, ExplosionTimer, Weapon, Ship}, GameTextures, EXPLOSION_LEN};
use bevy::prelude::Entity;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_event::<FireWeaponEvent>()
        .add_systems(Update, weapon_cooldown_system)
        .add_systems(Update, player_laser_hit_enemy_system)
        .add_systems(Update, enemy_laser_hit_player_system)
        .add_systems(Update, explosion_to_spawn_system)
        .add_systems(Update, explosion_animation_system)
        .add_systems(Update, try_fire_emp_listener)
        .add_systems(Update, ship_destroyed_system);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Component)]
pub enum WeaponType {
    Laser,
    EMP,
}

impl Default for WeaponType {
    fn default() -> Self {
        WeaponType::Laser
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Component)]
pub enum Allegiance {
    Friendly,
    Enemy,
}

impl Default for Allegiance {
    fn default() -> Self {
        Allegiance::Friendly
    }
}

#[derive(Event)]
pub struct FireWeaponEvent {
    pub weapon_type: WeaponType,
    pub weapon_alignment: Allegiance,
    pub firing_entity: Entity
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

            info!("Parent: {:?}, Firing Entity: {:?}", parent.get(), fire_event.firing_entity);
            if parent.get() != fire_event.firing_entity {
                continue;
            }

            info!("Current charge: {}", fired_weapon.current_charge);

            if fired_weapon.current_charge < 1. {
                continue;
            }

            let mut firing_xy: Option<(f32,f32)> = None;
            let mut firing_ship_allegiance: Option<Allegiance> = None;

            for (ship, allegiance, tf) in ship_query.iter_mut() {
                info!("Ship: {:?}", allegiance);
            }

            if let Ok((firing_ship, fsa, firing_ship_tf)) = ship_query.get_mut(fire_event.firing_entity) {
                fired_weapon.current_charge = 0.;
                firing_xy = Some((firing_ship_tf.translation.x, firing_ship_tf.translation.y));
                firing_ship_allegiance = Some(fsa.clone());

                info!("Firing ship: {:?}", fired_weapon.current_charge);

            } else {
                continue;
            }

            info!("Firing EMP");

            if let (Some((x, y)), Some(fsa)) = (firing_xy, firing_ship_allegiance) {
                // deal damage to enemy ships inversely proportional to distance
                for (mut ship, ship_allegiance, ship_tf) in ship_query.iter_mut() {
                    if *ship_allegiance == fsa {
                        continue;
                    }

                    let (x2, y2) = (ship_tf.translation.x, ship_tf.translation.y);
                    let distance = ((x2 - x).powi(2) + (y2 - y).powi(2)).sqrt();

                    let damage = 100. / distance;

                    ship.shields -= damage;

                    info!(ship.shields);
                }
                
            }
        }
            


            
        // }


        // if weapon_state. < 1. {
        //     continue;
        // }
    
        // let (x, y) = (player_tf.translation.x, player_tf.translation.y);
    
        // commands
        //     .spawn(SpriteBundle {
        //         texture: game_textures.player_laser.clone(),
        //         sprite: Sprite {
        //             color: Color::rgb(5.0, 5.0, 5.0),
        //             ..Default::default()
        //         },
        //         transform: Transform {
        //             translation: Vec3::new(x + x_offset, y, 0.),
        //             scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
        //             rotation: player_tf.rotation
        //         },
        //         ..Default::default()
        //     })
        //     .insert(Laser)
        //     .insert(FromPlayer)
        //     .insert(SpriteSize::from(PLAYER_LASER_SIZE))
        //     .insert(Movable { auto_despawn: true })
        //     .insert(Velocity { x: velocity.x, y: velocity.y, omega: 0.});
        
    
    
        // player_state.weapon_cooldown = player_state.weapon_cooldown_max;
    

    }

	
}

#[allow(clippy::type_complexity)] // for the Query types.
fn player_laser_hit_enemy_system(
	mut commands: Commands,
	mut enemy_count: ResMut<EnemyCount>,
	mut player_state: ResMut<PlayerState>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromPlayer>)>,
	enemy_query: Query<(Entity, &Transform, &SpriteSize), With<Enemy>>,
) {
	let mut despawned_entities: HashSet<Entity> = HashSet::new();

	// iterate through the lasers
	for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
		if despawned_entities.contains(&laser_entity) {
			continue;
		}

		let laser_scale = laser_tf.scale.xy();

		// iterate through the enemies
		for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
			if despawned_entities.contains(&enemy_entity)
				|| despawned_entities.contains(&laser_entity)
			{
				continue;
			}

			let enemy_scale = enemy_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				enemy_tf.translation,
				enemy_size.0 * enemy_scale,
			);

			// perform collision
			if collision.is_some() {
				// remove the enemy
				commands.entity(enemy_entity).despawn();
				despawned_entities.insert(enemy_entity);
				enemy_count.0 -= 1;
				player_state.score += 1;

				commands.spawn((ExplosionToSpawn {
					transform: Transform {
						translation: enemy_tf.translation,
						..Default::default()
					},
					duration: 0.05,
					is_engine: false
				},));
			}
		}
	}
}



#[allow(clippy::type_complexity)] // for the Query types.
fn enemy_laser_hit_player_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromEnemy>)>,
	player_query: Query<(Entity, &Transform, &SpriteSize), With<Player>>,
) {
	if let Ok((player_entity, player_tf, player_size)) = player_query.get_single() {
		let player_scale = player_tf.scale.xy();

		for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
			let laser_scale = laser_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				player_tf.translation,
				player_size.0 * player_scale,
			);

			// perform the collision
			if collision.is_some() {
				// remove the player
				commands.entity(player_entity).despawn_recursive();
				player_state.shot(time.elapsed_seconds_f64());
				player_state.score = 0;

				// remove the laser
				commands.entity(laser_entity).despawn();

				// spawn the explosionToSpawn
				commands.spawn((ExplosionToSpawn {
					transform: Transform {
						translation: player_tf.translation,
						..Default::default()
					},
					duration: 0.05,
					is_engine: false
				},));

				break;
			}
		}
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

        if ship.shields <= 0. {
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