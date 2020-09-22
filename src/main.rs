use clap::{App, Arg};
use dkregistry::v2::Client;
use std::fs::File;
use std::path::Path;
use tokio::runtime::Runtime;

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

enum State {
    ProcessingRepo(String),
    Individual,
}

#[derive(Debug)]
struct Image {
    repo: String,
    tag: String,
}

impl Image {
    fn new(repo: &str, tag: &str) -> Self {
        Image {
            repo: repo.to_owned(),
            tag: tag.to_owned(),
        }
    }
    fn from_str(inp: &str) -> Self {
        log::trace!("Image::from_str called with {}", inp);
        if let Some(ix) = inp.find(':') {
            Image {
                repo: inp[..ix].to_owned(),
                tag: inp[ix + 1..].to_owned(),
            }
        } else {
            panic!("No tag found in {}", inp);
        }
    }
    fn to_str(&self) -> String {
        [&self.repo, ":", &self.tag].concat()
    }
}

#[derive(Debug)]
enum Item {
    Tagged(String, Image),
    Untagged(Image),
}

impl Item {
    fn new(val: &str) -> Self {
        log::trace!("Item::new called with {}", val);
        match val.find('=') {
            Some(ix) => {
                log::trace!("Item has tag ending at {}", ix);
                Item::Tagged(val[..ix].to_owned(), Image::from_str(&val[ix + 1..]))
            }
            None => {
                log::trace!("Item has no tag");
                Item::Untagged(Image::from_str(val))
            }
        }
    }

    fn with_repo(repo: &str, val: &str) -> Self {
        if val.find(':').is_some() {
            panic!("Colon found in tag '{}'", val);
        }
        match val.find('=') {
            Some(ix) => Item::Tagged(val[..ix].to_owned(), Image::new(repo, &val[ix + 1..])),
            None => Item::Untagged(Image::new(repo, val)),
        }
    }

    fn image(&self) -> &Image {
        match self {
            Item::Tagged(_, i) => i,
            Item::Untagged(i) => i,
        }
    }

    fn name(&self) -> String {
        match self {
            Item::Tagged(name, _) => name.to_owned(),
            Item::Untagged(im) => im.to_str(),
        }
    }
}

fn start(first: &str) -> (State, Vec<Item>) {
    log::trace!("Start called with {}", first);
    let mut res = Vec::new();
    match first.find(':') {
        Some(_) => {
            log::trace!("Start contains tag");
            res.push(Item::new(first));
            (State::Individual, res)
        }
        None => {
            log::trace!("Start is repo only");
            if first.find('=').is_some() {
                panic!("First item is not a full image but has a tag: {}", first);
            }
            (State::ProcessingRepo(first.to_owned()), res)
        }
    }
}

fn next(state: &State, item: &str) -> Item {
    match state {
        State::ProcessingRepo(repo) => {
            if item.find(':').is_some() {
                panic!("Found repo specified in subsequent argument '{}' when processing with fixed repo ({})", item, repo);
            } else {
                Item::with_repo(repo, item)
            }
        }
        State::Individual => {
            if !item.find(':').is_some() {
                panic!("Argument is not provided with a tag '{}'", item);
            }
            Item::new(item)
        }
    }
}

fn process_args(mut args: clap::Values<'_>) -> Vec<Item> {
    let fst = args.next().unwrap();
    let (state, mut res) = start(fst);
    for item in args {
        res.push(next(&state, item));
    }
    res
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let matches = App::new("Build State Tool")
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
        )
        .get_matches();
    let registry = matches.value_of("registry").unwrap();
    let config = matches.value_of("docker-config").map(Path::new);
    let args = matches.values_of("images").unwrap();
    let items = process_args(args);

    let mut runtime = Runtime::new().unwrap();
    let result =
        runtime.block_on(async { find_first_existing_image(registry, config, items).await })?;
    match result {
        Some(item) => print!("{}", item.name()),
        None => (),
    }
    Ok(())
}
