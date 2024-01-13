use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::{PlayerState, CodePilotCode, components::{CodePilotActiveText, ScoreText, WeaponChargeBar}, UiState};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, ui_setup_system)
        .add_systems(Update, egui_system)
        .add_systems(Update, ui_update_system);
    }
}




fn ui_setup_system (
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
) { 

    //Setup the HUD
	//Text in the top left to show current score
    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::from_style(
                TextStyle {
                    font_size: 30.0,
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
                    color: Color::GOLD,
                    ..default()
			}),
        ]).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
        ScoreText,
    ));

	//Text in the bottom right to show whether Codepilot is running
	commands.spawn((
		// Create a TextBundle that has a Text with a list of sections.
		TextBundle::from_sections([
			TextSection::new(
				"Codepilot: ",
				TextStyle {
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
					font_size: 20.0,
					..default()
				},
			),
			TextSection::from_style(
				TextStyle {
					font_size: 20.0,
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
					color: Color::RED,
					..default()
			}),
		])
		.with_text_alignment(TextAlignment::Left)
		.with_style(Style {
			position_type: PositionType::Absolute,
			bottom: Val::Px(10.0),
			right: Val::Px(350.0),
			..default()
		}),
		CodePilotActiveText,
	));

	commands
    .spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
			position_type: PositionType::Absolute,
            justify_content: JustifyContent::FlexStart,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        ..default()
	}).with_children(|parent| {
		spawn_bar(parent, asset_server);
	});
}

fn egui_system(
	mut ui_state: ResMut<UiState>,
	mut contexts: EguiContexts) {
	let ctx = contexts.ctx_mut();

    egui::SidePanel::right("right_panel")
	.min_width(300.0)
	.show(ctx, |ui| {
        ui.vertical(|ui| {
			ui.label("Add Codepilot Code: ");
			ui.code_editor(&mut ui_state.player_code);
		});
    });
}

fn spawn_bar(parent: &mut ChildBuilder, asset_server: Res<AssetServer>) {
    parent
        .spawn(NodeBundle {
            style: Style {
				padding: UiRect::all(Val::Px(20.)),
                height: Val::Px(30.0),
                width: Val::Px(400.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {

            parent.spawn(TextBundle::from_section(
				"Weapon Charge:",
				TextStyle {
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
					font_size: 20.0,
					..default()
				}));

            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(100.),
                        height: Val::Px(10.),
                        padding: UiRect::all(Val::Px(1.)),
                        align_items: AlignItems::Stretch,
                        top: Val::Px(2.0),
                        left: Val::Px(6.0),
                        ..Default::default()
                    },
                    background_color: Color::BLACK.into(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                width : Val::Percent(50.0),
                                ..Default::default()
                            },
                            background_color: Color::GREEN.into(),
                            ..Default::default()
                        },
                        WeaponChargeBar,
                    ));
                });
        });
}

fn ui_update_system(
    player_state: Res<PlayerState>,
	copilotcode: Res<CodePilotCode>,
    mut scorequery: Query<&mut Text, (Without<CodePilotActiveText>, With<ScoreText>)>,
	mut codepilotquery: Query<&mut Text,  (With<CodePilotActiveText>, Without<ScoreText>)>,
    mut chargebarquery: Query<(&mut Style, &mut BackgroundColor), With<WeaponChargeBar>>,
) {
	//Update the Score
    for mut text in &mut scorequery {
        // Update the value of the second section
		text.sections[1].value = format!("{0}", player_state.score);
    }

	//Display whether Codepilot is running
	for mut text in codepilotquery.iter_mut() {
		if copilotcode.compiled.is_some() {
			text.sections[1].value = format!("Active");
			text.sections[1].style.color = Color::GREEN;
		} else {
			text.sections[1].value = format!("Disabled");
			text.sections[1].style.color = Color::RED;
		}
	}

    for (mut style, mut color) in chargebarquery.iter_mut() {
		color.0 = Color::rgb(1.0 * (player_state.weapon_cooldown / player_state.weapon_cooldown_max), 1.0 * (1. - player_state.weapon_cooldown / player_state.weapon_cooldown_max), 0.2);
		
		style.width = Val::Percent(100.0 * (1.0 - player_state.weapon_cooldown / player_state.weapon_cooldown_max));
	}
}