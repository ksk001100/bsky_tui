use atrium_api::{com::atproto::repo::strong_ref, types::string::Language};
use eyre::{bail, eyre, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageSpec {
    pub path: String,
    pub alt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyPolicy {
    Everyone,
    Nobody,
    Rules(Vec<ReplyRule>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyRule {
    Followers,
    Following,
    Mentioned,
}

#[derive(Clone, Debug)]
pub struct PostDraft {
    pub text: String,
    pub images: Vec<ImageSpec>,
    pub link: Option<String>,
    pub quote: Option<strong_ref::MainData>,
    pub langs: Option<Vec<Language>>,
    pub labels: Vec<String>,
    pub reply_policy: ReplyPolicy,
}

pub fn parse_drafts(input: &str) -> Result<Vec<PostDraft>> {
    let parts = input
        .split("\n---\n")
        .map(parse_draft)
        .collect::<Result<Vec<_>>>()?;
    if parts.is_empty() || parts.len() > 25 {
        bail!("a thread must contain between 1 and 25 posts");
    }
    Ok(parts)
}

fn parse_draft(input: &str) -> Result<PostDraft> {
    let mut images = Vec::new();
    let mut link = None;
    let mut quote = None;
    let mut langs = None;
    let mut labels = Vec::new();
    let mut reply_policy = ReplyPolicy::Everyone;
    let mut text = Vec::new();

    for line in input.lines() {
        if let Some(value) = line.strip_prefix("!image ") {
            let (path, alt) = value.split_once('|').unwrap_or((value, ""));
            images.push(ImageSpec {
                path: path.trim().to_owned(),
                alt: alt.trim().to_owned(),
            });
        } else if let Some(value) = line.strip_prefix("!link ") {
            link = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("!quote ") {
            let (uri, cid) = value
                .split_once('|')
                .ok_or_else(|| eyre!("!quote must use: !quote AT_URI | CID"))?;
            quote = Some(strong_ref::MainData {
                uri: uri.trim().to_owned(),
                cid: cid.trim().parse().map_err(eyre::Report::msg)?,
            });
        } else if let Some(value) = line.strip_prefix("!lang ") {
            let parsed = value
                .split(',')
                .map(|lang| Language::new(lang.trim().to_owned()).map_err(eyre::Report::msg))
                .collect::<Result<Vec<_>>>()?;
            langs = Some(parsed);
        } else if let Some(value) = line.strip_prefix("!label ") {
            labels.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|label| !label.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(value) = line.strip_prefix("!replies ") {
            reply_policy = parse_reply_policy(value.trim())?;
        } else {
            text.push(line);
        }
    }

    if images.len() > 4 {
        bail!("a post can contain at most 4 images");
    }
    if images.iter().any(|image| image.path.is_empty()) {
        bail!("!image requires a file path");
    }
    if images.is_empty() && link.is_none() && quote.is_none() && text.join("\n").trim().is_empty() {
        bail!("a post must contain text or an embed");
    }
    if !images.is_empty() && link.is_some() {
        bail!("images and an external link card cannot be combined in one post");
    }

    Ok(PostDraft {
        text: text.join("\n"),
        images,
        link,
        quote,
        langs,
        labels,
        reply_policy,
    })
}

fn parse_reply_policy(value: &str) -> Result<ReplyPolicy> {
    match value {
        "everyone" => Ok(ReplyPolicy::Everyone),
        "none" => Ok(ReplyPolicy::Nobody),
        _ => value
            .split(',')
            .map(str::trim)
            .map(|rule| match rule {
                "followers" => Ok(ReplyRule::Followers),
                "following" => Ok(ReplyRule::Following),
                "mentioned" => Ok(ReplyRule::Mentioned),
                _ => bail!("unknown reply rule: {rule}"),
            })
            .collect::<Result<Vec<_>>>()
            .map(ReplyPolicy::Rules),
    }
}

pub fn summary(drafts: &[PostDraft]) -> String {
    let images = drafts.iter().map(|draft| draft.images.len()).sum::<usize>();
    let links = drafts.iter().filter(|draft| draft.link.is_some()).count();
    let quotes = drafts.iter().filter(|draft| draft.quote.is_some()).count();
    format!(
        "{} post(s), {} image(s), {} link card(s), {} quote(s)",
        drafts.len(),
        images,
        links,
        quotes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thread_and_metadata_directives() {
        let drafts = parse_drafts(
            "!image /tmp/a.png | alt text\n!lang ja\nfirst\n---\n!replies followers,mentioned\nsecond",
        )
        .expect("drafts parse");
        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].images[0].alt, "alt text");
        assert_eq!(
            drafts[1].reply_policy,
            ReplyPolicy::Rules(vec![ReplyRule::Followers, ReplyRule::Mentioned])
        );
    }

    #[test]
    fn rejects_more_than_four_images() {
        let input = (0..5)
            .map(|index| format!("!image /tmp/{index}.png | alt"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse_drafts(&input).is_err());
    }
}
