use bevy::{prelude::*, utils::HashSet, sprite::collide_aabb::collide};

use crate::{PlayerState, WinSize, EnemyCount, components::{SpriteSize, Laser, FromPlayer, Enemy, FromEnemy, Player, ExplosionToSpawn, Explosion, ExplosionTimer}, GameTextures, EXPLOSION_LEN};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, weapon_cooldown_system)
            .add_systems(Update, player_laser_hit_enemy_system)
            .add_systems(Update, enemy_laser_hit_player_system)
            .add_systems(Update, explosion_to_spawn_system)
            .add_systems(Update, explosion_animation_system);
    }
}

fn weapon_cooldown_system(
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	win_size: Res<WinSize>,
) {
	if player_state.weapon_cooldown > 0. {
		player_state.weapon_cooldown -= time.delta_seconds();

		if player_state.weapon_cooldown <= 0. {
			info!("Ready to fire!");
		} 
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
				commands.entity(player_entity).despawn();
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