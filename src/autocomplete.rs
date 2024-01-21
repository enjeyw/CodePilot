use std::collections::HashMap;

use rustpython_parser::{self as parser};
use parser::{parse, Mode, ast::{Mod, self}};

type FunctionSuggestions = HashMap<String, Vec<String>>;

#[derive(Debug)]
pub struct AutoCompletes {
    classes: HashMap<String, FunctionSuggestions>,
    functions: FunctionSuggestions,
    assigments: HashMap<String, String>,
}

pub fn suggest_completions(input: &str, source: &str) -> Vec<String> {
    let available_autocompletes = get_available_autocompletes(source);

    //iterate through the available autocompletes and return the ones that match the input
    let mut suggestions: Vec<String> = Vec::new();

    match available_autocompletes {
        Some(available_autocompletes) => {
            let classes = available_autocompletes.classes;
            let functions = available_autocompletes.functions;
            let assignments = available_autocompletes.assigments;

            //check if the input is a class
            let class_names = classes.keys().collect::<Vec<_>>();
            for class_name in class_names {
                if class_name.starts_with(input) {
                    suggestions.push(class_name.to_string());
                }
            }

            //check if the input is a function
            let function_names = functions.keys().collect::<Vec<_>>();
            for function_name in function_names {
                if function_name.starts_with(input) {
                    // push the function with all the args to the suggestions

                    let function_args = functions.get(function_name).unwrap();
                    let mut function_with_args = function_name.to_string();

                    function_with_args.push('(');
                    function_with_args.push_str(&function_args.join(", "));
                    function_with_args.push(')');
                    suggestions.push(function_with_args);

                }
            }

            //check if the input is an assignment
            let assignment_names = assignments.keys().collect::<Vec<_>>();
            for assignment_name in assignment_names {

                // split the input by the dot, if it has one
                let split_input = input.split('.').collect::<Vec<_>>();
                let input_start = split_input[0];
                let input_start = dbg!(input_start);
                if split_input.len() == 2 {
                    // if the input starts with an assignment, check if the assignment maps to a class
                    // if it does, add the class functions that match the input end the suggestions
                    let input_end = split_input[1];

                   
                    if assignment_name == input_start {
                        let assignment_value = assignments.get(assignment_name).unwrap();
                        let class_functions = classes.get(assignment_value);

                        if let Some(class_functions) = class_functions {
                            let mut class_function_names = class_functions.keys().collect::<Vec<_>>();
                            class_function_names.sort();
                            
                            for class_function_name in class_function_names {
                                if class_function_name.starts_with(input_end) && class_function_name != "__init__" {
                                    let class_function_args = class_functions.get(class_function_name).unwrap();
                                    // filter out the 'self' argument
                                    let class_function_args = class_function_args.iter().filter(|arg: &&String| **arg != "self").map(|arg| arg.to_string()).collect::<Vec<_>>();        

                                    let mut class_function_with_args = format!(".{}", class_function_name);
                                    class_function_with_args.push('(');
                                    class_function_with_args.push_str(&class_function_args.join(", "));
                                    class_function_with_args.push(')');
                                    suggestions.push(class_function_with_args);
                                }
                            }
                        }
                    }
                } else if assignment_name == input_start {
                    // if the input exactly matches an assignment, check if the assignment maps to a class
                    // if it does, add the class functions to the suggestions

                    let assignment_value = assignments.get(assignment_name).unwrap();
                    let class_functions = classes.get(assignment_value);

                    if let Some(class_functions) = class_functions {
                        let mut class_function_names = class_functions.keys().collect::<Vec<_>>();
                        class_function_names.sort();

                        for class_function_name in class_function_names {
                            if class_function_name != "__init__"  {
                                let class_function_args = class_functions.get(class_function_name).unwrap();
                                // filter out the 'self' argument
                                let class_function_args = class_function_args.iter().filter(|arg: &&String| **arg != "self").map(|arg| arg.to_string()).collect::<Vec<_>>();

                                let mut class_function_with_args = format!(".{}", class_function_name);
                                class_function_with_args.push('(');
                                class_function_with_args.push_str(&class_function_args.join(", "));
                                class_function_with_args.push(')');
                                suggestions.push(class_function_with_args);
                                
                            }
                            
                        }
                    }

                } else if assignment_name.starts_with(input) {
                    suggestions.push(assignment_name.to_string());
                }

                
            }

        },
        None => {}
    }
    suggestions
}


// removes the line in a string for a given character index
fn remove_line(input_sting: &str, index: usize, offset: usize) -> String {
    let mut line_count = 0;
    for (i, c) in input_sting.chars().enumerate() {
        if c == '\n' {
            line_count += 1;
        }
    
        if i == index {
            break;
        }
    }

    let mut lines = input_sting.lines().collect::<Vec<_>>();
    if offset > line_count - 1 || line_count - offset >= lines.len() {
        return String::new()
    }


	lines.remove(line_count - offset);
	lines.join("\n")
}

fn strip_parse(source: &str) -> Option<Mod> {

    let program = parse(source,  Mode::Module,  "<embedded>");

    match program {
        Ok(prog) => {
            return Some(prog)
        },
        Err(err) => {
            let offset = err.offset;
            // println!("Failed to parse: {}", err);

            let shortened = remove_line(source, offset.to_usize(), 0);


            if shortened.len() == 0 {
                return None
            }

            println!("Attempting to parse: {}", shortened.clone());

            return strip_parse(&shortened);
        }
        
    }

}

fn get_available_autocompletes(source: &str) -> Option<AutoCompletes> {

    let mut classes: HashMap<String, FunctionSuggestions> = HashMap::new();
    let mut functions: FunctionSuggestions = HashMap::new();
    let mut assigments: HashMap<String, String> = HashMap::new();


    let program = strip_parse(source);

    match program {
        Some(prog) => {
            match prog {
                Mod::Module(mod_module) => {

                    let body = mod_module.body;

                    body.iter().for_each(|stmt| {
                        match stmt {
                            ast::Stmt::ClassDef(class_def) => {
                                let mut class_functions: FunctionSuggestions = HashMap::new();

                                class_def.body.iter().for_each(|stmt| {
                                    match stmt {
                                        ast::Stmt::FunctionDef(func_def) => {
                                            let function_name = func_def.name.as_str().to_owned();
                                            let function_args = func_def.args.args.iter().map(
                                                |arg| arg.def.arg.as_str().to_owned()
                                            ).collect::<Vec<_>>();

                                            class_functions.insert(function_name, function_args);
                                        },
                                        _ => {}
                                    }
                                });

                                classes.insert(class_def.name.as_str().to_owned(), class_functions);

                            },
                            ast::Stmt::FunctionDef(func_def) => {
                                let function_name = func_def.name.as_str().to_owned();
                                let function_args = func_def.args.args.iter().map(
                                    |arg| arg.def.arg.as_str().to_owned()
                                ).collect::<Vec<_>>();

                                functions.insert(function_name, function_args.clone());

                            },
                            ast::Stmt::Assign(assign) => {
                                let assign_target = assign.targets[0].clone().to_string();
                                let v = *assign.value.clone();
                                match v {
                                    ast::Expr::Call(call) => {
                                        let func = *call.func;
                                        let assinged_to = func.to_string();

                                        assigments.insert(assign_target, assinged_to);

                                    },
                                    _ => {
                                        assigments.insert(assign_target, String::new());

                                    }
                                }
                                
                            },
                            _ => {}
                        }
                    });


                },
                _ => {
                    println!("No body: {}", source);
                    return None
                }
            }
            
            
        },
        None => {
            println!("Failed to parse: {}", source);
            return None
        }
        
    }

    Some(AutoCompletes {
        classes: classes,
        functions: functions,
        assigments: assigments,
    })
    
}