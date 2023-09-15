
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod info;
mod package;
mod profile;
mod sdk;
mod template;
mod util;
mod index;
mod file;
mod indexer;
mod project;

use util::*;

/// Command-line interface for Sapphire
#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
	#[clap(subcommand)]
	command: SapphireCommands,
}

#[derive(Subcommand, Debug)]
enum SapphireCommands {
	/// Initialize a new Sapphire project
	New {
		/// The target directory to create the project in
		path: Option<PathBuf>
	},

	/// Options for managing profiles (installations of Sapphire)
	Profile {
		#[clap(subcommand)]
		commands: crate::profile::Profile,
	},

	/// Options for configuring Sapphire CLI
	Config {
		#[clap(subcommand)]
		commands: crate::info::Info,
	},

	/// Options for installing & managing the Sapphire SDK
	Sdk {
		#[clap(subcommand)]
		commands: crate::sdk::Sdk,
	},

	/// Tools for working with the current mod project
	Project {
		#[clap(subcommand)]
		commands: crate::project::Project,
	},

	/// Options for working with . packages
	Package {
		#[clap(subcommand)]
		commands: crate::package::Package,
	},

	/// Tools for interacting with the Sapphire mod index
	Index {
		#[clap(subcommand)]
		commands: crate::index::Index,
	},

	/// Run default instance of Geometry Dash
	Run {
		/// Run Geometry Dash in the background instead of the foreground
		#[clap(long)]
		background: bool
	}
}

fn main() {
	#[cfg(windows)]
	match ansi_term::enable_ansi_support() {
		Ok(_) => {},
		Err(_) => println!("Unable to enable color support, output may look weird!")
	};

	let args = Args::parse();

	let mut config = config::Config::new();

	match args.command {
		SapphireCommands::New { path } => template::build_template(&mut config, path),
		SapphireCommands::Profile { commands } => profile::subcommand(&mut config, commands),
		SapphireCommands::Config { commands } => info::subcommand(&mut config, commands),
		SapphireCommands::Sdk { commands } => sdk::subcommand(&mut config, commands),
		SapphireCommands::Package { commands } => package::subcommand(&mut config, commands),
		SapphireCommands::Project { commands } => project::subcommand(&mut config, commands),
		SapphireCommands::Index { commands } => index::subcommand(&mut config, commands),
		SapphireCommands::Run { background } => profile::run_profile(&config, None, background)
	}

	config.save();
}
