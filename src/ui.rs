use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui::{self, Pos2, text_edit::{CCursorRange, CursorRange}, text::CCursor, epaint::text::cursor::Cursor, TextEdit}};

use egui_extras::syntax_highlighting::highlight;

use crate::{PlayerState, CodePilotCode, components::{CodePilotActiveText, ScoreText, WeaponChargeBar}, autocomplete};

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
	mut codepilot_code: ResMut<CodePilotCode>,
	mut contexts: EguiContexts) {
	let ctx = contexts.ctx_mut();

    // Load these once at the start of your program
    egui::SidePanel::right("right_panel")
    	.min_width(400.0)
    	.show(ctx, |ui| {
                
            ui.vertical(|ui| {
    			ui.label("Add Codepilot Code: ");

                
                let language = "py";
                let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());

                let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                    let layout_job = highlight(ui.ctx(), &theme, string, language);
                    // layout_job.wrap.max_width = wrap_width; // no wrapping
                    ui.fonts(|f| f.layout_job(layout_job))
                };

                // https://github.com/emilk/egui/blob/ccbddcfe951e01c55efd0ed19f2f2ab5edfad5d9/egui_demo_lib/src/apps/demo/text_edit.rs

                let prev_raw_code = codepilot_code.raw_code.clone();
                let prev_cursor_index = codepilot_code.cursor_index.clone();

                let mut ccursor_adjustment: isize = 0;

                // If we escape autocomplete, we need to regain focus on the text box
                let mut escaped = false;

                let completions_len = codepilot_code.completions.len();
                if completions_len > 0 {

                    if ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) { 
                        escaped = true;
                        codepilot_code.completions = Vec::new();
                    }
                    
                    if ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) { 
                        codepilot_code.selected_completion = (codepilot_code.selected_completion + 1) % completions_len;
                    }

                    if ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) { 
                        codepilot_code.selected_completion = (codepilot_code.selected_completion - 1) % completions_len;
                    }

                    if let Some(cursor_index) = codepilot_code.cursor_index {
                        if ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) ||
                            ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)){ 
                        
                            let completion = codepilot_code.completions[codepilot_code.selected_completion].clone();
    
                            //we need to strip the part of the completion that is identical
                            let mut autocomp_token_len = codepilot_code.autocomplete_token.len();
    
                            //now handle the special case of autocompletion of a class function, where only after the dot will be filled
                            if completion.starts_with(".") {
                                let split_input = codepilot_code.autocomplete_token.split('.').collect::<Vec<_>>();
                                let split_input_len = split_input.len();
    
                                if split_input_len == 1 {
                                    //no dot present in input token, which means we're at the end of the class assignment
                                    autocomp_token_len = 0;
                                } else {
                                    autocomp_token_len = split_input[split_input_len - 1].len() + 1;
                                }
    
                            }
                           
                            let (first, last) = prev_raw_code.split_at(cursor_index);
                                let mut new_code: String = first.to_owned();
                                let completion_to_insert = &completion.as_str()[autocomp_token_len..];
                                new_code.push_str(completion_to_insert);
                                new_code.push_str(last);
    
                                codepilot_code.raw_code = new_code;
    
                                ccursor_adjustment = completion_to_insert.len() as isize;
                        } 
                    }
                }

                let newline_requested = ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                let backspace_requested = ui.input_mut(|i: &mut egui::InputState| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace));

                let mut output = egui::TextEdit::multiline(&mut codepilot_code.raw_code)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .desired_rows(10)
                .desired_width(400.)
                .lock_focus(true)
                .layouter(&mut layouter)
                .show(ui);

                let mut response = output.response;

                if escaped {
                    response.request_focus();
                }

                let mut loc = response.rect.left_top();                
                loc.x += 3.;

                if let Some(text_cursor_range) = output.cursor_range {
                    let cindex: usize = text_cursor_range.primary.ccursor.index;

                    codepilot_code.cursor_index =Some(cindex);

                    let cursor_row = text_cursor_range.primary.rcursor.row;
                    let cursor_col = text_cursor_range.primary.rcursor.column;

                    // split the head on tabs, spaces or newlines
                    let head: &str = &codepilot_code.raw_code.clone()[..cindex];
                    let mut head = head.split(|c| c == '\t' || c == ' ' || c == '\n').collect::<Vec<_>>();

                    if prev_raw_code != codepilot_code.raw_code || codepilot_code.cursor_index != prev_cursor_index {
                        if let Some(last) = head.pop() {
                            if last != "" {
                                let completions = autocomplete::suggest_completions(last, &codepilot_code.raw_code);
                                codepilot_code.completions = completions;
                                codepilot_code.autocomplete_token = last.to_owned();
                                codepilot_code.selected_completion = 0;
                            } else {
                                codepilot_code.completions = Vec::new();
                                codepilot_code.selected_completion = 0;
                                codepilot_code.autocomplete_token = String::new();
                            }
                        }
                    }

                    loc.x += 7. * cursor_col as f32;
                    loc.y += 14. * (cursor_row as f32 + 1.);

                    if codepilot_code.completions.len() > 0 {
                        let completions = codepilot_code.completions.clone();
                        egui::Window::new("Codepilot")
                            .fixed_pos(loc)
                            .title_bar(false)
                            .show(ctx, |ui| {
                                for (idx, completion) in completions.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut codepilot_code.selected_completion,
                                        idx,
                                        completion
                                        );
                                }
                            });
                    }

                    if newline_requested {
                        // add a newline to the code, with the same indentation as the previous line
                        let code_lines = codepilot_code.raw_code.split('\n').collect::<Vec<_>>();
                        let active_line = code_lines[cursor_row].to_owned();
                        
                        //supports tab indendation and space indentation but not both at the same time b
                        let mut total_tabs_at_start_of_active_line = active_line.chars().take_while(|c: &char| *c == '\t').count();
                        let mut total_spaces_at_start_of_active_line = active_line.chars().take_while(|c: &char| *c ==' ').count();

                        if active_line.ends_with(':') || active_line.ends_with('{') || active_line.ends_with('[') || active_line.ends_with('(') {
                            if total_tabs_at_start_of_active_line > 0 {
                                total_tabs_at_start_of_active_line += 1
                            } else {
                                total_spaces_at_start_of_active_line += 4
                            }
                        }

                        let indent = "\n".to_owned() + &"\t".repeat(total_tabs_at_start_of_active_line) + &" ".repeat(total_spaces_at_start_of_active_line);

                        codepilot_code.raw_code.insert_str(cindex, &indent);

                        ccursor_adjustment += (1 + total_tabs_at_start_of_active_line + total_spaces_at_start_of_active_line) as isize;   
                    }

                    if backspace_requested {
                        // if the previous characters are 4X spaces, remove all 4 (it's an indent), otherwise just do a regular backspace 

                        if codepilot_code.raw_code.clone()[..cindex].ends_with("    ") {
                            for _ in 0..4 {
                                codepilot_code.raw_code.remove(cindex - 4);
                                ccursor_adjustment -= 1;
                            };
                        } else {
                            codepilot_code.raw_code.remove(cindex - 1);
                            ccursor_adjustment -= 1;
                        }
                    }

                    if let Some(mut state) = TextEdit::load_state(ui.ctx(),  response.id) {
                        if let Some(mut ccursor_range) = state.ccursor_range() {
                            if ccursor_adjustment != 0 {
                                ccursor_range.primary.index = (ccursor_range.primary.index as isize + ccursor_adjustment) as usize;
                                ccursor_range.secondary = ccursor_range.primary;
                                state.set_ccursor_range(Some(ccursor_range));
                                state.store(ui.ctx(), response.id);   
                            }
                        }
                    }
                };
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
