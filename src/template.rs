use colored::*;
use crate::GEODE_VERSION;
use crate::print_error;
use git2::Repository;
use path_absolutize::Absolutize;
use rustyline::Editor;
use serde_json::{json, to_string_pretty};
use std::path::PathBuf;
use std::{fs, path::Path, process::exit};
use std::process::Command;
use std::env;
use std::process;

use fs_extra::dir as fs_dir;

pub fn create_template(mut project_name: String, location: Option<PathBuf>) {
	let is_location_default = location.is_none();
	let loc = match location {
	    Some(s) => s,
	    None => std::env::current_dir().unwrap()
	};

	let mut version = String::from("v1.0.0");
	let mut developer = String::from("");
	let mut description = String::from("");
	let mut buffer = if is_location_default {
		loc.absolutize().unwrap().join(&project_name).to_str().unwrap().to_string()
	} else {
		loc.absolutize().unwrap().to_str().unwrap().to_string()
	};

	let mut rl = Editor::<()>::new();

	let mut prompts = [
	    ("Developer", &mut developer, true),
	    ("Version", &mut version, true),
	    ("Description", &mut description, true),
	    ("Location", &mut buffer, true),
	];
	
	for (prompt, ref mut var, required) in prompts.iter_mut() {
	    let text = format!("{}: ", prompt);

		loop {
			let readline = rl.readline_with_initial(text.as_str(), (var.as_str(), ""));
			match readline {
				Ok(line) => {
					rl.add_history_entry(line.as_str());
					if line.is_empty() && *required {
						println!("{}", "Please enter a value".red());
					} else {
						**var = line;
						break;
					}
				},
				Err(err) => {
					print_error!("Error: {}", err);
				}
			}
		}
	}
	
	buffer = buffer.trim().to_string();
	version = version.trim().to_string();
	developer = developer.trim().to_string();
	project_name = project_name.trim().to_string();
	description = description.trim().to_string();

	let project_location = Path::new(&buffer);

	let id = format!("com.{}.{}", developer.to_lowercase(), project_name.to_lowercase());

	let mut binary_name = project_name.to_lowercase();
	binary_name.retain(|c| !c.is_whitespace());
	
	println!(
	    "Creating mod with ID {} named {} by {} version {} in {}",
	    id.green(),
	    project_name.green(),
	    developer.green(),
	    version.green(),
	    project_location.to_str().unwrap().green()
	);

	if project_location.exists() {
	    println!("{}", "Unable to create project in existing directory".red());
	    exit(1);
	} else {
		let result = fs::create_dir_all(&project_location);
		if result.is_err() {
			println!("{}", "Unable to create directory for project".red());
			exit(1);
		}
	}

	match Repository::clone("https://github.com/geode-sdk/example-mod", &project_location) {
	    Ok(_) => (),
	    Err(e) => print_error!("failed to clone template: {}", e),
	};

	fs::remove_dir_all(&project_location.join(".git")).unwrap();

	for thing in fs::read_dir(&project_location).unwrap() {
	    if !thing.as_ref().unwrap().metadata().unwrap().is_dir() {
	        let file = thing.unwrap().path();
	        let contents = fs::read_to_string(&file).unwrap().replace("$Template", &project_name);

	        fs::write(file, contents).unwrap();
	    }
	}         

	let tmp_sdk = std::env::temp_dir().join("sdk");

	if tmp_sdk.exists() {
	    fs_dir::remove(&tmp_sdk).unwrap();
	}

	match Repository::clone_recurse("https://github.com/geode-sdk/sdk", &tmp_sdk) {
	    Ok(_) => (),
	    Err(e) => print_error!("failed to clone sdk: {}", e),
	};

	copy_dir::copy_dir(&tmp_sdk, project_location.join("sdk")).unwrap();
	fs_dir::remove(tmp_sdk).unwrap();

	
	let mod_json = json!({
	    "geode":        GEODE_VERSION,
	    "version":      version,
	    "id":           id,
	    "name":         project_name,
	    "developer":    developer,
	    "description":  description,
	    "details":      null,
	    "credits":      null,
	    "binary": {
	        "*": binary_name
	    },
	    "dependencies": [
	        {
	            "id": "com.geode.api",
	            "required": true
	        }
	    ]
	});

	fs::write(
	    &project_location.join("mod.json"),
	    to_string_pretty(&mod_json).unwrap()
	).expect("Unable to write to specified project");
	
	let location_str = project_location.parent().unwrap().to_str().unwrap();

	if cfg!(windows)
	{
		let mut set_up_project = String::new();
		println!("Would you like to set up and open the project? (y/n):");
		let _answer = std::io::stdin().read_line(&mut set_up_project).unwrap();

		let mod_folder = format!("{}/{}", &location_str, &project_name);
	
		if set_up_project.trim() == "y"
		{
			let mut ide = String::new();
			println!("Select Compatible IDE: \n1. VS Code.\n2. Visual Studio.");
			let _ide_answer = std::io::stdin().read_line(&mut ide).unwrap();
	
			let build_folder_in_mod = format!("{}/build", &mod_folder);
			
			if ide.trim() == "1" // Open VS Code
			{
				assert!(env::set_current_dir(&mod_folder).is_ok());
				Command::new("cmd").arg("/c").arg("code").arg(".").spawn().expect("Uh oh!");
			}
			else if ide.trim() == "2" // Open and Set Up Visual Studio
			{
				std::fs::create_dir(&build_folder_in_mod).unwrap();
				assert!(env::set_current_dir(&build_folder_in_mod).is_ok());
				//println!("Successfully changed working directory to {}!", env::current_dir().unwrap().into_os_string().into_string().unwrap());
	
				let mut cmake = Command::new("cmake").arg("..").arg("-A").arg("Win32").spawn().expect("Uh oh!");
				let _end = cmake.wait().unwrap();
				let sln_file = format!("{}.sln", project_name);
				println!("Opening Visual Studio Solution...");
				Command::new("cmd").arg("/c").arg(sln_file).spawn().expect("Uh oh!");
			}
		}
		else if set_up_project.trim() == "n"
		{
			assert!(env::set_current_dir(&mod_folder).is_ok());
			Command::new("explorer").arg(".").spawn().expect("Uh oh!");
		}
	}

	process::exit(0);
}