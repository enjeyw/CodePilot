use std::collections::HashMap;

use bevy::{prelude::*, utils::HashSet, sprite::{collide_aabb::collide, MaterialMesh2dBundle, Mesh2dHandle}, render::mesh};

use rustpython_vm as vm;
use vm::PyObjectRef;
use vm::builtins::PyList;
use rustpython::vm::{
    pyclass, pymodule, PyObject, PyPayload, PyResult, TryFromBorrowedObject, VirtualMachine, stdlib
};
use vm::convert::ToPyObject;

use crate::{CodePilotCode, GameTextures, PlayerState, components::{Velocity, Player, Enemy}, player::{try_fire_weapon, accelerate_counter_clockwise, accelerate_clockwise, accelerate_forward, accelerate_backward}, events::CompileCodeEvent, CommandState};

macro_rules! add_python_function {
    ( $scope:ident, $vm:ident, $src:literal $(,)? ) => {{
        // compile the code to bytecode
        let code = vm::py_compile!(source = $src);
        // convert the rustpython_compiler_core::CodeObject to a PyRef<PyCode>
        let code = $vm.ctx.new_code(code);

        // run the python code in the scope to store the function
        $vm.run_code_obj(code, $scope.clone())
    }};
}

// removes the line in a string for a given index
fn remove_line(input_sting: &str, line_index: usize) -> String {
	let mut lines = input_sting.lines().collect::<Vec<_>>();
	lines.remove(line_index);
	lines.join("\n")
}


pub struct CodePilotPlugin;

impl Plugin for CodePilotPlugin {
	fn build(&self, app: &mut App) {
		app
        .add_systems(Update, player_codepilot_compile_system)
        .add_systems(Update, codepilot_event_system);

	}
}

#[derive(Debug, Clone)]
struct PyAccessibleV3Vec(Vec<Vec3>);
impl ToPyObject for PyAccessibleV3Vec {
	fn to_pyobject(self, vm: &VirtualMachine) -> PyObjectRef {
		let list: Vec<PyObjectRef>= self.0.into_iter().map(
			|e| {
				vm.new_pyobj((e.x, e.y, e.z))
			} 
		).collect();
		PyList::new_ref(list, vm.as_ref()).to_pyobject(vm)
	}
}
fn player_codepilot_compile_system(
	mut commands: Commands,
	mut compile_code_event: EventReader<CompileCodeEvent>,
	mut codepilot_code: ResMut<CodePilotCode>,
) {
	for ev in compile_code_event.read() {
		let interpreter = rustpython::InterpreterConfig::new()
				.init_stdlib()
				.interpreter();

		let code_obj = interpreter.enter(|vm| {
			let scope: vm::scope::Scope = vm.new_scope_with_builtins();

			let source = codepilot_code.raw_code.as_str();
			
			let code_obj_res = vm
				.compile(source, vm::compiler::Mode::Exec, "<embedded>".to_owned())
				.map_err(|err| vm.new_syntax_error(&err, Some(source)));

			match code_obj_res {
				Ok(code_obj) => {
					codepilot_code.compiled = Some(code_obj);
				}
				Err(exc) => {
					let mut s = String::new();
					vm.write_exception(&mut s, &exc);
					codepilot_code.py_result = Some(s);
				}
			}
		});
    }
}

fn try_boolean_python_action (key: &str, scope: &vm::scope::Scope, vm: &VirtualMachine) -> bool {
	let fire = scope.globals.get_item(key, vm);
				
	if let Ok(fire_ref) = fire {
		let fire_bool_res = fire_ref.is_true(vm);
		if let Ok(fire_bool) = fire_bool_res {
			info!("fire: {}", fire_bool);

			if fire_bool {
				return true;
			}
		}
	}

		return false;
}

fn codepilot_event_system(
	mut commands: Commands,
	kb: Res<Input<KeyCode>>,
	mut codepilot_code: ResMut<CodePilotCode>,
	game_textures: Res<GameTextures>,
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	mut query: Query<(&mut Velocity, &Transform), With<Player>>,
	enemy_query: Query<(&Velocity, &Transform), (Without<Player>, With<Enemy>)>,
) {
	let acceleration = 0.05;
	let ang_acceleration = 0.005;

	let max_speed = 2.0;
	let max_ang_velocity = 0.5;
	

	if let Ok((mut velocity, transform)) = query.get_single_mut() {

		let heading_vec = transform.rotation * Vec3::X;
		let heading_angle = transform.rotation.mul_vec3(Vec3::X).y.atan2(transform.rotation.mul_vec3(Vec3::X).x);
		let heading_perp = transform.rotation * Vec3::Y;
		let speed = velocity.y.hypot(velocity.x);
		let course = (velocity.y).atan2(velocity.x);		

		// ensure speed is not greater than max speed
		if speed > max_speed {
			velocity.x = course.cos() * max_speed;
			velocity.y = course.sin() * max_speed;
		}

		// Convert all enemy velocity and positions to lists
		let enemy_velocities: PyAccessibleV3Vec = PyAccessibleV3Vec(
			enemy_query.iter().map(|(vel, _)| Vec3::new(vel.x, vel.y, vel.omega)).collect()
		);

		let enemy_positions: PyAccessibleV3Vec = PyAccessibleV3Vec(
			enemy_query.iter().map(|(_, transform)| {
				//get the enemy heading as f32 radians
				let enemy_heading = transform.rotation.mul_vec3(Vec3::X).y.atan2(transform.rotation.mul_vec3(Vec3::X).x);

				Vec3::new(transform.translation.x, transform.translation.y, enemy_heading)
			}).collect()
		);

		// Codepilot player control section
		if let Some(cpc) = codepilot_code.compiled.clone() {

			let interpreter = rustpython::InterpreterConfig::new()
				.init_stdlib()
				.interpreter();

			// let interpreter: vm::prelude::Interpreter = vm::Interpreter::without_stdlib(Default::default());

			interpreter.enter(|vm | {
				let scope = vm.new_scope_with_builtins();

				// Set the player position
				scope
					.globals
					.set_item("player_position", vm.new_pyobj((
						transform.translation.x,
						transform.translation.y,
						heading_vec[0],
						heading_vec[1],
						heading_angle
					)), vm);
				
				// Set the player velocity
				scope
					.globals
					.set_item("player_velocity", vm.new_pyobj((velocity.x, velocity.y, velocity.omega)), vm);

				scope
					.globals
					.set_item("enemy_positions", vm.new_pyobj(enemy_positions), vm);

				scope
					.globals
					.set_item("enemy_velocities", vm.new_pyobj(enemy_velocities), vm);

				let helper_code = vm::py_compile!(file = "./src/python_helpers_5.py");
		
				let helper_res = vm.run_code_obj(vm.ctx.new_code(helper_code), scope.clone());
				match helper_res {
					Ok(_) => {},
					Err(exc) =>  { 
						let mut s = String::new();
						vm.write_exception(&mut s, &exc);
						codepilot_code.py_result = Some(s);
					}
				}
				
				let player_code_res = vm.run_code_obj(cpc, scope.clone());

				match player_code_res {
					Ok(player_code_res) => { codepilot_code.py_result = None},
					Err(exc) =>  { 
						let mut s = String::new();
						vm.write_exception(&mut s, &exc);
						codepilot_code.py_result = Some(s);
					}
				}

				let mut command_state = CommandState::default();
			
				if try_boolean_python_action("fire", &scope, vm) {
					try_fire_weapon(&mut commands, &game_textures, &mut player_state, transform);
					command_state.fire = true;
				}

				if try_boolean_python_action("counterclockwise", &scope, vm) {
					accelerate_counter_clockwise(
						&mut velocity, transform, ang_acceleration, max_ang_velocity, heading_vec, heading_perp, &mut commands);
					command_state.counter_clockwise = true;
				}

				if try_boolean_python_action("clockwise", &scope, vm) {
					accelerate_clockwise(
						&mut velocity, transform, ang_acceleration, max_ang_velocity, heading_vec, heading_perp, &mut commands);
					command_state.clockwise = true;
				}

				if try_boolean_python_action("forward", &scope, vm) {
					accelerate_forward(
						&mut velocity, transform, acceleration, max_speed, heading_vec, heading_perp, &mut commands
					);
					command_state.forward = true;
				}

				if try_boolean_python_action("backward", &scope, vm) {
					accelerate_backward(
						&mut velocity, transform, acceleration, max_speed, heading_vec, heading_perp, &mut commands
					);
					command_state.backward = true;
				}

				let time = time.elapsed_seconds();

				if let Some(command) = codepilot_code.command_state_history.last() {
					let last_command_state = &command.1;
					if last_command_state != &command_state {
						codepilot_code.command_state_history.push((time, command_state));
					}
				} else {
					codepilot_code.command_state_history.push((time, command_state));
				};

				if codepilot_code.command_state_history.len() > 10 {
					codepilot_code.command_state_history.remove(0);
				}

			});
		}
	}
}

