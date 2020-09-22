use crate::images::*;

enum State {
    ProcessingRepo(String),
    Individual,
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

pub fn process_args(mut args: clap::Values<'_>) -> Vec<Item> {
    let fst = args.next().unwrap();
    let (state, mut res) = start(fst);
    for item in args {
        res.push(next(&state, item));
    }
    res
}
