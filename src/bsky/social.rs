//! social Bluesky services.

use super::*;

pub async fn post_thread(agent: &BskyAgent, uri: String) -> Result<get_post_thread::Output> {
    Ok(agent
        .api
        .app
        .bsky
        .feed
        .get_post_thread(
            get_post_thread::ParametersData {
                depth: Some(100_u16.try_into().map_err(eyre::Report::msg)?),
                parent_height: Some(100_u16.try_into().map_err(eyre::Report::msg)?),
                uri,
            }
            .into(),
        )
        .await?)
}

pub async fn post_likes(agent: &BskyAgent, uri: String, cid: Cid) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_likes(
            get_likes::ParametersData {
                cid: Some(cid),
                cursor: None,
                limit: None,
                uri,
            }
            .into(),
        )
        .await?;
    Ok(output
        .likes
        .iter()
        .map(|like| profile_interaction(&like.actor))
        .collect())
}

pub async fn post_reposts(
    agent: &BskyAgent,
    uri: String,
    cid: Cid,
) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_reposted_by(
            get_reposted_by::ParametersData {
                cid: Some(cid),
                cursor: None,
                limit: None,
                uri,
            }
            .into(),
        )
        .await?;
    Ok(output
        .reposted_by
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}

pub async fn post_quotes(agent: &BskyAgent, uri: String, cid: Cid) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_quotes(
            get_quotes::ParametersData {
                cid: Some(cid),
                cursor: None,
                limit: None,
                uri,
            }
            .into(),
        )
        .await?;
    Ok(output
        .posts
        .iter()
        .map(|post| {
            let record = post::Record::try_from_unknown(post.record.clone()).ok();
            InteractionItem {
                title: format!(
                    "{} @{} · {}",
                    post.author.display_name.clone().unwrap_or_default(),
                    post.author.handle.as_str(),
                    record
                        .as_ref()
                        .map(|record| utils::format_post_datetime(record.created_at.as_str()))
                        .unwrap_or_else(|| "time unavailable".to_owned())
                ),
                subtitle: record
                    .map(|record| record.text.clone())
                    .unwrap_or_else(|| "[Post unavailable]".to_owned()),
                url: get_url(post.author.handle.clone(), post.uri.clone()).unwrap_or_default(),
                actor: None,
            }
        })
        .collect())
}

pub(super) fn profile_interaction(
    profile: &atrium_api::app::bsky::actor::defs::ProfileViewData,
) -> InteractionItem {
    InteractionItem {
        title: profile.display_name.clone().unwrap_or_default(),
        subtitle: format!("@{}", profile.handle.as_str()),
        url: format!("https://bsky.app/profile/{}", profile.did.as_str()),
        actor: Some(profile.did.clone().into()),
    }
}

pub async fn profile(
    agent: &BskyAgent,
    actor: AtIdentifier,
) -> Result<atrium_api::app::bsky::actor::defs::ProfileViewDetailed> {
    Ok(agent
        .api
        .app
        .bsky
        .actor
        .get_profile(get_profile::ParametersData { actor }.into())
        .await?)
}

pub async fn profile_content(
    agent: &BskyAgent,
    actor: AtIdentifier,
    section: ProfileSection,
) -> Result<ProfileContent> {
    match section {
        ProfileSection::Posts | ProfileSection::Replies | ProfileSection::Media => {
            let filter = match section {
                ProfileSection::Posts => "posts_no_replies",
                ProfileSection::Replies => "posts_with_replies",
                ProfileSection::Media => "posts_with_media",
                _ => return Ok(ProfileContent::Items(Vec::new())),
            };
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_author_feed(
                    get_author_feed::ParametersData {
                        actor,
                        cursor: None,
                        filter: Some(filter.to_owned()),
                        include_pins: Some(true),
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Posts(output.feed.clone()))
        }
        ProfileSection::Likes => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_actor_likes(
                    get_actor_likes::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Posts(output.feed.clone()))
        }
        ProfileSection::Feeds => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_actor_feeds(
                    get_actor_feeds::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Items(
                output
                    .feeds
                    .iter()
                    .map(|feed| ProfileListItem {
                        title: feed.display_name.clone(),
                        subtitle: feed.description.clone().unwrap_or_default(),
                        url: at_uri_web_url(&feed.uri, "feed").unwrap_or_default(),
                    })
                    .collect(),
            ))
        }
        ProfileSection::Lists => {
            let output = agent
                .api
                .app
                .bsky
                .graph
                .get_lists(
                    get_lists::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                        purposes: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Items(
                output
                    .lists
                    .iter()
                    .map(|list| ProfileListItem {
                        title: list.name.clone(),
                        subtitle: list.description.clone().unwrap_or_else(|| {
                            format!("{} members", list.list_item_count.unwrap_or(0))
                        }),
                        url: at_uri_web_url(&list.uri, "lists").unwrap_or_default(),
                    })
                    .collect(),
            ))
        }
        ProfileSection::StarterPacks => {
            let output = agent
                .api
                .app
                .bsky
                .graph
                .get_actor_starter_packs(
                    get_actor_starter_packs::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Items(
                output
                    .starter_packs
                    .iter()
                    .map(|pack| {
                        let record =
                            starterpack::Record::try_from_unknown(pack.record.clone()).ok();
                        ProfileListItem {
                            title: record
                                .as_ref()
                                .map(|record| record.name.clone())
                                .unwrap_or_else(|| "Starter Pack".to_owned()),
                            subtitle: record
                                .and_then(|record| record.description.clone())
                                .unwrap_or_else(|| {
                                    format!("{} members", pack.list_item_count.unwrap_or(0))
                                }),
                            url: starter_pack_url(&pack.uri, pack.creator.did.as_str())
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
            ))
        }
    }
}

pub async fn toggle_follow(
    agent: &BskyAgent,
    profile: &atrium_api::app::bsky::actor::defs::ProfileViewDetailedData,
) -> Result<Option<String>> {
    if let Some(uri) = profile
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.following.as_ref())
    {
        agent.delete_record(uri).await?;
        Ok(None)
    } else {
        let output = agent
            .create_record(follow::RecordData {
                created_at: Datetime::now(),
                subject: profile.did.clone(),
            })
            .await?;
        Ok(Some(output.uri.clone()))
    }
}

pub async fn moderate(agent: &BskyAgent, action: ModerationAction) -> Result<()> {
    match action {
        ModerationAction::MuteActor { did, muted } => {
            if muted {
                agent
                    .api
                    .app
                    .bsky
                    .graph
                    .unmute_actor(unmute_actor::InputData { actor: did.into() }.into())
                    .await?;
            } else {
                agent
                    .api
                    .app
                    .bsky
                    .graph
                    .mute_actor(mute_actor::InputData { actor: did.into() }.into())
                    .await?;
            }
        }
        ModerationAction::BlockActor { did, blocking_uri } => {
            if let Some(uri) = blocking_uri {
                agent.delete_record(uri).await?;
            } else {
                agent
                    .create_record(block::RecordData {
                        created_at: Datetime::now(),
                        subject: did,
                    })
                    .await?;
            }
        }
        ModerationAction::ReportActor(did) => {
            agent
                .api
                .com
                .atproto
                .moderation
                .create_report(
                    moderation::create_report::InputData {
                        mod_tool: None,
                        reason: Some("Reported from bsky_tui".to_owned()),
                        reason_type: moderation::defs::REASON_OTHER.to_owned(),
                        subject: Union::Refs(
                            moderation::create_report::InputSubjectRefs::ComAtprotoAdminDefsRepoRef(
                                Box::new(RepoRefData { did }.into()),
                            ),
                        ),
                    }
                    .into(),
                )
                .await?;
        }
        ModerationAction::ReportPost { uri, cid } => {
            agent
                .api
                .com
                .atproto
                .moderation
                .create_report(
                    moderation::create_report::InputData {
                        mod_tool: None,
                        reason: Some("Reported from bsky_tui".to_owned()),
                        reason_type: moderation::defs::REASON_OTHER.to_owned(),
                        subject: Union::Refs(
                            moderation::create_report::InputSubjectRefs::ComAtprotoRepoStrongRefMain(
                                Box::new(repo::strong_ref::MainData { uri, cid }.into()),
                            ),
                        ),
                    }
                    .into(),
                )
                .await?;
        }
    }
    Ok(())
}

pub(super) fn at_uri_parts(uri: &str) -> Option<(&str, &str)> {
    let mut parts = uri.strip_prefix("at://")?.split('/');
    let did = parts.next()?;
    let _collection = parts.next()?;
    let rkey = parts.next()?;
    Some((did, rkey))
}

pub(super) fn at_uri_web_url(uri: &str, kind: &str) -> Option<String> {
    let (did, rkey) = at_uri_parts(uri)?;
    Some(format!("https://bsky.app/profile/{did}/{kind}/{rkey}"))
}

pub(super) fn starter_pack_url(uri: &str, creator: &str) -> Option<String> {
    let (_, rkey) = at_uri_parts(uri)?;
    Some(format!("https://bsky.app/starter-pack/{creator}/{rkey}"))
}

pub async fn search(
    agent: &BskyAgent,
    query: String,
    cursor: Option<String>,
) -> Result<search_posts::Output> {
    let search_result = agent
        .api
        .app
        .bsky
        .feed
        .search_posts(
            search_posts::ParametersData {
                cursor,
                limit: None,
                q: query.clone(),
                author: None,
                domain: None,
                lang: None,
                mentions: None,
                since: None,
                sort: Some("latest".to_string()),
                tag: None,
                until: None,
                url: None,
            }
            .into(),
        )
        .await?;

    Ok(search_result)
}

pub async fn search_users(agent: &BskyAgent, query: String) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .actor
        .search_actors(
            search_actors::ParametersData {
                cursor: None,
                limit: None,
                q: Some(query),
                term: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .actors
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}

pub async fn followers(agent: &BskyAgent, actor: AtIdentifier) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_followers(
            get_followers::ParametersData {
                actor,
                cursor: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .followers
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}

pub async fn follows(agent: &BskyAgent, actor: AtIdentifier) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_follows(
            get_follows::ParametersData {
                actor,
                cursor: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .follows
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}
