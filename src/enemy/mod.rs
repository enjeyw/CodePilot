use self::formation::{Formation, FormationMaker};
use crate::components::{Enemy, FromEnemy, Laser, Movable, SpriteSize, Velocity, Player};
use crate::{
	EnemyCount, GameTextures, WinSize, ENEMY_LASER_SIZE, ENEMY_MAX, ENEMY_SIZE, SPRITE_SCALE,
};

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use rand::{thread_rng, Rng};
use std::{f32::consts::PI, time::Duration};

mod formation;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(FormationMaker::default())
			.add_systems(Update, enemy_spawn_system.run_if(on_timer(Duration::from_secs(1))))
			.add_systems(Update, enemy_fire_system.run_if(enemy_fire_criteria))
			.add_systems(Update, enemy_movement_system);
	}
}

fn enemy_spawn_system(
	mut commands: Commands,
	game_textures: Res<GameTextures>,
	mut enemy_count: ResMut<EnemyCount>,
	mut formation_maker: ResMut<FormationMaker>,
	win_size: Res<WinSize>,
) {
	if enemy_count.0 < ENEMY_MAX {
		// get formation and start x/y
		let formation = formation_maker.make(&win_size);
		let (x, y) = formation.start;

		commands
			.spawn(SpriteBundle {
				texture: game_textures.enemy.clone(),
				transform: Transform {
					translation: Vec3::new(x, y, 10.),
					scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
					..Default::default()
				},
				..Default::default()
			})
			.insert(Enemy)
			.insert(Movable { auto_despawn: false })
			.insert(Velocity { x: 1., y: 0., omega: 0.})
			.insert(formation)
			.insert(SpriteSize::from(ENEMY_SIZE));

		enemy_count.0 += 1;
	}
}

fn enemy_fire_criteria() -> bool {
	thread_rng().gen_bool(1. / 30000.)
}

fn enemy_fire_system(
	mut commands: Commands,
	game_textures: Res<GameTextures>,
	enemy_query: Query<&Transform, With<Enemy>>,
) {
	for &tf in enemy_query.iter() {
		let velocity = tf.rotation * Vec3::X * 3.0;
		let (x, y) = (tf.translation.x, tf.translation.y);
		// spawn enemy laser sprite
		commands
			.spawn(SpriteBundle {
				texture: game_textures.enemy_laser.clone(),
				transform: Transform {
					translation: Vec3::new(x, y - 15., 0.),
					rotation: tf.rotation,
					scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
				},
				..Default::default()
			})
			.insert(Laser)
			.insert(SpriteSize::from(ENEMY_LASER_SIZE))
			.insert(FromEnemy)
			.insert(Movable { auto_despawn: true })
			.insert(Velocity { x: velocity.x, y: velocity.y , omega: 0.});
	}
}

fn enemy_movement_system(
	time: Res<Time>,
	mut query: Query<(&mut Velocity, &mut Transform, &mut Formation),  Without<Player>>,
	player_query: Query<&Transform, With<Player>>,
) {
	let delta = time.delta_seconds();

	let acceleration = 0.5;
	let ang_acceleration = 0.1;

	let max_speed = 2.0;
	let max_ang_velocity = 0.18;


	if let Ok(player_transform) = player_query.get_single() {
		let player_translation: Vec2 = player_transform.translation.xy();

		for (mut velocity, mut transform, mut formation) in &mut query {

			let speed = velocity.y.hypot(velocity.x);
			let course = (velocity.y).atan2(velocity.x);

			if speed > max_speed {
				velocity.x = course.cos() * max_speed;
				velocity.y = course.sin() * max_speed;
			}

			if velocity.omega < -max_ang_velocity {
				velocity.omega = -max_ang_velocity;
			}

			if velocity.omega > max_ang_velocity {
				velocity.omega = max_ang_velocity;
			}
			
			let adjust_speed = thread_rng().gen_bool(0.1);
			let do_turn = thread_rng().gen_bool(0.001);

			let heading = transform.rotation * Vec3::X;

			let to_player = (player_translation - transform.translation.xy()).normalize();

			let heading_dot_player = heading.xy().dot(to_player);

			if (heading_dot_player - 1.0).abs() < f32::EPSILON {
				continue;
			}

			let enemy_right = (transform.rotation * Vec3::Y).xy();
			let right_dot_player = enemy_right.dot(to_player);

			velocity.omega += right_dot_player*ang_acceleration * 1.0;

			if adjust_speed {
				velocity.x += heading.x * acceleration;
				velocity.y += heading.y * acceleration;
			}

		}
	}

	// 	// ensure speed is not greater than max speed
	// 	let speed = velocity.y.hypot(velocity.x);
	// 	let course = (velocity.y).atan2(velocity.x);

	// 	if speed > max_speed {
	// 		velocity.x = course.cos() * max_speed;
	// 		velocity.y = course.sin() * max_speed;
	// 	}

	// 	if do_turn {
	// 		if thread_rng().gen_bool(0.5){
	// 			info!("turning pos x: {} y: {} o: {}", velocity.x, velocity.y, velocity.omega);

	// 			velocity.omega += ang_acceleration;

	// 			if velocity.omega > max_ang_velocity {
	// 				velocity.omega = max_ang_velocity;
	// 			}

	// 		} else {
	// 			info!("turning neg x: {} y: {} o: {}", velocity.x, velocity.y, velocity.omega);

	// 			velocity.omega -= ang_acceleration;

	// 			if velocity.omega < -max_ang_velocity {
	// 				velocity.omega = -max_ang_velocity;
	// 			}
	// 		}

	// 	}
		


	// 	// // current position
	// 	// let (x_org, y_org) = (transform.translation.x, transform.translation.y);

	// 	// // max distance
	// 	// let max_distance = delta * formation.speed;

	// 	// // 1 for counter clockwise, -1 clockwise
	// 	// let dir: f32 = if formation.start.0 < 0. { 1. } else { -1. };
	// 	// let (x_pivot, y_pivot) = formation.pivot;
	// 	// let (x_radius, y_radius) = formation.radius;

	// 	// // compute next angle (based on time for now)
	// 	// let angle = formation.angle
	// 	// 	+ dir * formation.speed * delta / (x_radius.min(y_radius) * PI / 2.);

	// 	// // compute target x/y
	// 	// let x_dst = x_radius * angle.cos() + x_pivot;
	// 	// let y_dst = y_radius * angle.sin() + y_pivot;

	// 	// // compute distance
	// 	// let dx = x_org - x_dst;
	// 	// let dy = y_org - y_dst;
	// 	// let distance = (dx * dx + dy * dy).sqrt();
	// 	// let distance_ratio = if distance == 0. { 0. } else { max_distance / distance };

	// 	// // compute final x/y
	// 	// let x = x_org - dx * distance_ratio;
	// 	// let x = if dx > 0. { x.max(x_dst) } else { x.min(x_dst) };
	// 	// let y = y_org - dy * distance_ratio;
	// 	// let y = if dy > 0. { y.max(y_dst) } else { y.min(y_dst) };

	// 	// // start rotating the formation angle only when sprite is on or close to ellipse
	// 	// if distance < max_distance * formation.speed / 20. {
	// 	// 	formation.angle = angle;
	// 	// }

	// 	// let translation = &mut transform.translation;
	// 	// (translation.x, translation.y) = (x, y);
	// }
}
