use std::{env, io::Read, sync::Arc};

use clap::Parser;
use cli::Args;
use download::download;
use inspect::{inspect_log, InspectType};
use install::install_packages;
use list::{list_installed_packages, list_packages, query_package, search_packages};
use logging::setup_logging;
use progress::create_progress_bar;
use remove::remove_packages;
use run::run_package;
use self_actions::process_self_action;
use soar_core::{
    config::generate_default_config,
    constants::{bin_path, cache_path, db_path, packages_path, repositories_path, root_path},
    database::packages::get_installed_packages,
    utils::setup_required_paths,
    SoarResult,
};
use soar_dl::downloader::{DownloadOptions, Downloader};
use tracing::{error, info};
use update::update_packages;

mod cli;
mod download;
mod inspect;
mod install;
mod list;
mod logging;
mod progress;
mod remove;
mod run;
mod self_actions;
mod state;
mod update;
mod utils;

async fn handle_cli() -> SoarResult<()> {
    let mut args = env::args().collect::<Vec<_>>();
    let self_bin = args.first().unwrap().clone();
    let self_version = env!("CARGO_PKG_VERSION");

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-" {
            let mut stdin = std::io::stdin();
            let mut buffer = String::new();
            if stdin.read_to_string(&mut buffer).is_ok() {
                let stdin_args = buffer.split_whitespace().collect::<Vec<&str>>();
                args.remove(i);
                args.splice(i..i, stdin_args.into_iter().map(String::from));
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    let args = Args::parse_from(args);

    setup_logging(&args);

    match args.command {
        cli::Commands::Install {
            packages,
            force,
            yes,
            portable,
            portable_home,
            portable_config,
        } => {
            if portable.is_some() && (portable_home.is_some() || portable_config.is_some()) {
                error!("--portable cannot be used with --portable-home or --portable-config");
                std::process::exit(1);
            }

            let portable = portable.map(|p| p.unwrap_or_default());
            let portable_home = portable_home.map(|p| p.unwrap_or_default());
            let portable_config = portable_config.map(|p| p.unwrap_or_default());

            install_packages(
                &packages,
                force,
                yes,
                portable,
                portable_home,
                portable_config,
            )
            .await?;
        }
        cli::Commands::Search {
            query,
            case_sensitive,
            limit,
        } => {
            search_packages(query, case_sensitive, limit).await?;
        }
        cli::Commands::Query { query } => {
            query_package(query).await?;
        }
        cli::Commands::Remove { packages, exact } => {
            remove_packages(&packages, exact).await?;
        }
        cli::Commands::Sync => unreachable!(),
        cli::Commands::Update { packages } => {
            update_packages(packages).await?;
        }
        cli::Commands::ListInstalledPackages {
            packages,
            repo_name,
        } => {
            list_installed_packages(repo_name).await?;
        }
        cli::Commands::ListPackages { repo_name } => {
            list_packages(repo_name).await?;
        }
        cli::Commands::Log { package } => inspect_log(&package, InspectType::BuildLog).await?,
        cli::Commands::Inspect { package } => {
            inspect_log(&package, InspectType::BuildScript).await?
        }
        cli::Commands::Run { yes, command } => {
            run_package(command.as_ref()).await?;
        }
        cli::Commands::Use { package } => unreachable!(),
        cli::Commands::Download {
            links,
            yes,
            output,
            regex_patterns,
            match_keywords,
            exclude_keywords,
            github,
            gitlab,
        } => {
            download(
                links,
                github,
                gitlab,
                regex_patterns,
                match_keywords,
                exclude_keywords,
                output,
                yes,
            )
            .await?;
        }
        cli::Commands::Health => unreachable!(),
        cli::Commands::DefConfig => generate_default_config()?,
        cli::Commands::Env => {
            info!("SOAR_ROOT={}", root_path().display());
            info!("SOAR_BIN={}", bin_path().display());
            info!("SOAR_DB={}", db_path().display());
            info!("SOAR_CACHE={}", cache_path().display());
            info!("SOAR_PACKAGE={}", packages_path().display());
            info!("SOAR_REPOSITORIES={}", repositories_path().display());
        }
        cli::Commands::Build { files } => unreachable!(),
        cli::Commands::SelfCmd { action } => {
            process_self_action(&action, self_bin, self_version).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    setup_required_paths().unwrap();

    if let Err(err) = handle_cli().await {
        error!("{}", err);
    };
}
