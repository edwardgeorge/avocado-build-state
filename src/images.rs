#[derive(Debug)]
pub struct Image {
    pub repo: String,
    pub tag: String,
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
pub enum Item {
    Tagged(String, Image),
    Untagged(Image),
}

impl Item {
    pub fn new(val: &str) -> Self {
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

    pub fn with_repo(repo: &str, val: &str) -> Self {
        if val.find(':').is_some() {
            panic!("Colon found in tag '{}'", val);
        }
        match val.find('=') {
            Some(ix) => Item::Tagged(val[..ix].to_owned(), Image::new(repo, &val[ix + 1..])),
            None => Item::Untagged(Image::new(repo, val)),
        }
    }

    pub fn image(&self) -> &Image {
        match self {
            Item::Tagged(_, i) => i,
            Item::Untagged(i) => i,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Item::Tagged(name, _) => name.to_owned(),
            Item::Untagged(im) => im.to_str(),
        }
    }
}
