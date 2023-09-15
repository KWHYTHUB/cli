use std::cell::{RefCell, Ref};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use crate::{done, fail, info, warn, NiceUnwrap};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Profile {
	pub name: String,
	pub gd_path: PathBuf,

	#[serde(flatten)]
	other: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
	pub current_profile: Option<String>,
	pub profiles: Vec<RefCell<Profile>>,
	pub default_developer: Option<String>,
	pub sdk_nightly: bool,
	#[serde(flatten)]
	other: HashMap<String, Value>,
}

// old config.json structures for migration
// TODO: remove this in 3.0
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OldConfigInstallation {
	pub path: PathBuf,
	pub executable: String,
}

// TODO: remove this in 3.0
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OldConfig {
	pub default_installation: usize,
	pub working_installation: Option<usize>,
	pub installations: Option<Vec<OldConfigInstallation>>,
	pub default_developer: Option<String>,
}

// TODO: remove this in 3.0
impl OldConfig {
	pub fn migrate(&self) -> Config {
		let profiles = self
			.installations
			.as_ref()
			.map(|insts| {
				insts
					.iter()
					.map(|inst| {
						RefCell::from(Profile {
							name: inst
								.executable
								.strip_suffix(".exe")
								.unwrap_or(&inst.executable)
								.into(),
							gd_path: inst.path.clone(),
							other: HashMap::new(),
						})
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		Config {
			current_profile: profiles
				.get(
					self.working_installation
						.unwrap_or(self.default_installation),
				)
				.map(|i| i.borrow().name.clone()),
			profiles,
			default_developer: self.default_developer.to_owned(),
			sdk_nightly: false,
			other: HashMap::new(),
		}
	}
}

pub fn _root() -> PathBuf {
	// get data dir per-platform
	let data_dir: PathBuf;
	#[cfg(any(windows, target_os = "linux"))]
	{
		data_dir = dirs::data_local_dir().unwrap().join("Sapphire");
	};
	#[cfg(target_os = "macos")]
	{
		data_dir = PathBuf::from("/Users/Shared/Sapphire");
	};
	#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
	{
		use std::compile_error;
		compile_error!("implement root directory");
	};
	data_dir
}

fn migrate_location(name: &str, mut path: PathBuf) -> PathBuf {
	// Migrate folder to executable
	if cfg!(windows) && path.is_dir() {
		path.push("GeometryDash.exe");

		if !path.exists() {
			warn!("Unable to find GeometryDash.exe in profile \
				  '{}', please update the GD path for it", name);
		}
	} else if path.file_name().unwrap() == "Contents" {
		path = path.parent().unwrap().to_path_buf();
	}

	path
}

impl Profile {
	pub fn new(name: String, location: PathBuf) -> Profile {
		Profile {
			gd_path: migrate_location(&name, location),
			name,
			other: HashMap::<String, Value>::new(),
		}
	}

	pub fn _dir(&self) -> PathBuf {
		if cfg!(windows) {
			self.gd_path.parent().unwrap().join("")
		} else {
			self.gd_path.join("Contents/")
		}
	}

	pub fn index_dir(&self) -> PathBuf {
		self._dir().join("index")
	}

	pub fn mods_dir(&self) -> PathBuf {
		self._dir().join("mods")
	}
}

impl Config {
	pub fn get_profile(&self, name: &Option<String>) -> Option<&RefCell<Profile>> {
		if let Some(name) = name {
			self.profiles.iter().find(|x| &x.borrow().name == name)
		} else {
			None
		}
	}

	pub fn get_current_profile(&self) -> Ref<Profile> {
		self
			.get_profile(&self.current_profile)
			.nice_unwrap("No current profile found!")
			.borrow()
	}

	pub fn try_sdk_path() -> Result<PathBuf, String> {
		let sdk_var = std::env::var("SAPPHIRE_SDK")
			.map_err(|_|
				"Unable to find Sapphire SDK (SAPPHIRE_SDK isn't set). Please install \
				it using ` sdk install` or use ` sdk set-path` to set \
				it to an existing clone. If you just installed the SDK using \
				` sdk install`, please restart your terminal / computer to \
				apply changes."
			)?;
	
		let path = PathBuf::from(sdk_var);
		if !path.is_dir() {
			return Err(format!(
				"Internal Error: SAPPHIRE_SDK doesn't point to a directory ({}). This \
				might be caused by having run ` sdk set-path` - try restarting \
				your terminal / computer, or reinstall using ` sdk install --reinstall`",
				path.display()
			));
		}
		if !path.join("VERSION").exists() {
			return Err(
				"Internal Error: SAPPHIRE_SDK/VERSION not found. Please reinstall \
				the Sapphire SDK using ` sdk install --reinstall`".into()
			);
		}
	
		Ok(path)
	}

	pub fn sdk_path() -> PathBuf {
		Self::try_sdk_path().nice_unwrap("Unable to get SDK path")
	}

	pub fn new() -> Config {
		if !_root().exists() {
			warn!("It seems you don't have Sapphire installed. Some operations will not work");
			warn!("You can setup Sapphire using ` config setup`");

			return Config {
				current_profile: None,
				profiles: Vec::new(),
				default_developer: None,
				sdk_nightly: false,
				other: HashMap::<String, Value>::new(),
			};
		}

		let config_json = _root().join("config.json");

		let mut output: Config = if !config_json.exists() {
			info!("Setup Sapphire using ` config setup`");
			// Create new config
			Config {
				current_profile: None,
				profiles: Vec::new(),
				default_developer: None,
				sdk_nightly: false,
				other: HashMap::<String, Value>::new(),
			}
		} else {
			// Parse config
			let config_json_str =
				&std::fs::read_to_string(&config_json).nice_unwrap("Unable to read config.json");
			match serde_json::from_str(config_json_str) {
				Ok(json) => json,
				Err(e) => {
					// Try migrating old config
					// TODO: remove this in 3.0
					let json = serde_json::from_str::<OldConfig>(config_json_str)
						.ok()
						.nice_unwrap(format!("Unable to parse config.json: {}", e));
					info!("Migrating old config.json");
					json.migrate()
				}
			}
		};

		output.save();

		if output.profiles.is_empty() {
			warn!("No Sapphire profiles found! Some operations will be unavailable.");
			warn!("Setup Sapphire using ` config setup`");
		} else if output.get_profile(&output.current_profile).is_none() {
			output.current_profile = Some(output.profiles[0].borrow().name.clone());
		}

		output
	}

	pub fn save(&self) {
		std::fs::create_dir_all(_root()).nice_unwrap("Unable to create Sapphire directory");
		std::fs::write(
			_root().join("config.json"),
			serde_json::to_string(self).unwrap(),
		)
		.nice_unwrap("Unable to save config");
	}

	pub fn rename_profile(&mut self, old: &str, new: String) {
		let profile = self
			.get_profile(&Some(String::from(old)))
			.nice_unwrap(&format!("Profile named '{}' does not exist", old));

		if self.get_profile(&Some(new.to_owned())).is_some() {
			fail!("The name '{}' is already taken!", new);
		} else {
			done!("Successfully renamed '{}' to '{}'", old, &new);
			profile.borrow_mut().name = new;
		}
	}
}
