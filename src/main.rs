use clap::{App, AppSettings, Arg, SubCommand};
use dkregistry::v2::Client;
use std::fs::File;
use std::path::Path;
use tokio::runtime::Runtime;

mod args;
mod images;
use crate::args::process_args;
use crate::images::Item;

const REGISTRY_CRED_USR: &str = "REGISTRY_CRED_USR";
const REGISTRY_CRED_PSW: &str = "REGISTRY_CRED_PSW";

async fn create_and_auth_client(
    registry_host: &str,
    config_file: Option<&Path>,
    repos: &[&str],
) -> anyhow::Result<Client> {
    let mut config = Client::configure()
        .registry(registry_host)
        .username(std::env::var(REGISTRY_CRED_USR).ok())
        .password(std::env::var(REGISTRY_CRED_PSW).ok());
    if let Some(cfg_path) = config_file {
        let f = File::open(cfg_path)?;
        config = config.read_credentials(f);
    }
    let mut client = config.build()?;
    let x: Vec<_> = repos
        .iter()
        .map(|repo| ["repository:", repo, ":pull"].concat())
        .collect();
    let y: Vec<_> = x.iter().map(String::as_str).collect();
    client = client
        .authenticate(&y)
        // .authenticate(&["repository:*:pull"])
        .await?;
    Ok(client)
}

async fn find_first_existing_image(
    registry_host: &str,
    config_file: Option<&Path>,
    mut items: Vec<Item>,
) -> anyhow::Result<Option<Item>> {
    log::debug!("Querying for images: {:?}", items);
    let repos: Vec<&str> = items.iter().map(|v| v.image().repo.as_ref()).collect();
    let client = create_and_auth_client(registry_host, config_file, &repos).await?;
    for item in items.drain(..) {
        let image = item.image();
        if client
            .has_manifest(&image.repo, &image.tag, None)
            .await?
            .is_some()
        {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let matches = App::new("Build State Tool")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("query-registry")
                .arg(
                    Arg::with_name("docker-config")
                        .short("c")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("registry")
                        .long("registry")
                        .short("r")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("images")
                        .index(1)
                        .required(true)
                        .multiple(true)
                        .min_values(1),
                ),
        )
        .get_matches();
    if let Some(m) = matches.subcommand_matches("query-registry") {
        let registry = m.value_of("registry").unwrap();
        let config = m.value_of("docker-config").map(Path::new);
        let args = m.values_of("images").unwrap();
        let items = process_args(args);

        let mut runtime = Runtime::new().unwrap();
        let result =
            runtime.block_on(async { find_first_existing_image(registry, config, items).await })?;
        match result {
            Some(item) => print!("{}", item.name()),
            None => (),
        }
        Ok(())
    } else {
        panic!("Unknown subcommand");
    }
}
