use std::convert::TryInto;

use atrium_api::{
    app::bsky::{
        actor::{defs::PreferencesItem, get_preferences, put_preferences},
        graph::{
            defs, get_actor_starter_packs, get_list, get_lists, list, listitem, mute_actor_list,
            starterpack, unmute_actor_list,
        },
        unspecced::{get_suggested_users, get_trending_topics},
    },
    chat::bsky::convo::{
        defs as convo_defs, get_convo_for_members, get_messages, list_convos, mute_convo,
        send_message, unmute_convo, update_read,
    },
    types::{
        string::{Datetime, Did, RecordKey},
        TryFromUnknown, Union,
    },
};
use bsky_sdk::{record::Record, BskyAgent};
use eyre::{bail, eyre, Result};

use crate::app::feature_panel::ReportSubject;
use crate::app::feature_panel::{FeatureRow, FeatureTarget};

const CHAT_PROXY: &str = "did:web:api.bsky.chat#bsky_chat";

pub async fn own_lists(agent: &BskyAgent, did: Did) -> Result<Vec<FeatureRow>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_lists(
            get_lists::ParametersData {
                actor: did.clone().into(),
                cursor: None,
                limit: None,
                purposes: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .lists
        .iter()
        .map(|list| list_row(list, &did))
        .collect())
}

pub async fn list_detail(agent: &BskyAgent, uri: String, did: Did) -> Result<Vec<FeatureRow>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_list(
            get_list::ParametersData {
                cursor: None,
                limit: None,
                list: uri.clone(),
            }
            .into(),
        )
        .await?;
    let owned = output.list.creator.did == did;
    Ok(output
        .items
        .iter()
        .map(|item| FeatureRow {
            title: format!(
                "{} @{}",
                item.subject.display_name.as_deref().unwrap_or(""),
                item.subject.handle.as_str()
            ),
            detail: item.subject.description.clone().unwrap_or_default(),
            target: FeatureTarget::ListMember {
                list_uri: uri.clone(),
                item_uri: item.uri.clone(),
                actor: item.subject.did.clone().into(),
            },
            unread: false,
        })
        .chain(std::iter::once(FeatureRow {
            title: format!("List feed · {}", output.list.name),
            detail: if owned {
                "Enter to use this list as the home feed; n/a/e/x manage it".into()
            } else {
                "Enter to use this list as the home feed".into()
            },
            target: FeatureTarget::List {
                uri: uri.clone(),
                cid: output.list.cid.clone(),
                purpose: output.list.purpose.clone(),
                owned,
                muted: output
                    .list
                    .viewer
                    .as_ref()
                    .and_then(|viewer| viewer.muted)
                    .unwrap_or(false),
            },
            unread: false,
        }))
        .collect())
}

pub async fn list_overview(agent: &BskyAgent, uri: String, did: &Did) -> Result<FeatureRow> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_list(
            get_list::ParametersData {
                cursor: None,
                limit: Some(1_u8.try_into().map_err(eyre::Report::msg)?),
                list: uri,
            }
            .into(),
        )
        .await?;
    Ok(list_row(&output.list, did))
}

fn list_row(list: &defs::ListView, did: &Did) -> FeatureRow {
    let muted = list
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.muted)
        .unwrap_or(false);
    FeatureRow {
        title: format!(
            "{}{}",
            if list.purpose == defs::MODLIST {
                "🛡 "
            } else {
                "☷ "
            },
            list.name
        ),
        detail: format!(
            "{} members · @{}{}\n{}",
            list.list_item_count.unwrap_or(0),
            list.creator.handle.as_str(),
            if muted { " · subscribed" } else { "" },
            list.description.as_deref().unwrap_or("")
        ),
        target: FeatureTarget::List {
            uri: list.uri.clone(),
            cid: list.cid.clone(),
            purpose: list.purpose.clone(),
            owned: &list.creator.did == did,
            muted,
        },
        unread: false,
    }
}

pub async fn create_list(
    agent: &BskyAgent,
    purpose: String,
    name: String,
    description: Option<String>,
) -> Result<String> {
    if name.trim().is_empty() {
        bail!("list name cannot be empty");
    }
    let purpose = match purpose.to_ascii_lowercase().as_str() {
        "mod" | "moderation" => defs::MODLIST.to_owned(),
        "curation" | "curate" => defs::CURATELIST.to_owned(),
        value if value == defs::MODLIST || value == defs::CURATELIST => value.to_owned(),
        _ => bail!("purpose must be curation or moderation"),
    };
    let output = list::RecordData {
        avatar: None,
        created_at: Datetime::now(),
        description,
        description_facets: None,
        labels: None,
        name,
        purpose,
    }
    .create(agent)
    .await?;
    Ok(output.uri.clone())
}

pub async fn edit_list(
    agent: &BskyAgent,
    uri: &str,
    purpose: String,
    name: String,
    description: Option<String>,
) -> Result<()> {
    let rkey = rkey(uri)?;
    list::RecordData {
        avatar: None,
        created_at: Datetime::now(),
        description,
        description_facets: None,
        labels: None,
        name,
        purpose,
    }
    .put(agent, rkey)
    .await?;
    Ok(())
}

pub async fn add_list_member(agent: &BskyAgent, list: String, subject: Did) -> Result<()> {
    listitem::RecordData {
        created_at: Datetime::now(),
        list,
        subject,
    }
    .create(agent)
    .await?;
    Ok(())
}

pub async fn delete_record(agent: &BskyAgent, uri: &str) -> Result<()> {
    agent.delete_record(uri).await?;
    Ok(())
}

pub async fn toggle_moderation_list(agent: &BskyAgent, uri: String, muted: bool) -> Result<()> {
    if muted {
        agent
            .api
            .app
            .bsky
            .graph
            .unmute_actor_list(unmute_actor_list::InputData { list: uri }.into())
            .await?;
    } else {
        agent
            .api
            .app
            .bsky
            .graph
            .mute_actor_list(mute_actor_list::InputData { list: uri }.into())
            .await?;
    }
    Ok(())
}

pub async fn toggle_thread_mute(agent: &BskyAgent, root: String, muted: bool) -> Result<()> {
    if muted {
        agent
            .api
            .app
            .bsky
            .graph
            .unmute_thread(atrium_api::app::bsky::graph::unmute_thread::InputData { root }.into())
            .await?;
    } else {
        agent
            .api
            .app
            .bsky
            .graph
            .mute_thread(atrium_api::app::bsky::graph::mute_thread::InputData { root }.into())
            .await?;
    }
    Ok(())
}

pub async fn toggle_hidden_reply(
    agent: &BskyAgent,
    root: &atrium_api::app::bsky::feed::defs::PostViewData,
    reply: String,
) -> Result<()> {
    use atrium_api::app::bsky::feed::threadgate;
    let mut record = root
        .threadgate
        .as_ref()
        .and_then(|view| view.record.clone())
        .and_then(|record| threadgate::Record::try_from_unknown(record).ok())
        .map(|record| record.data)
        .unwrap_or(threadgate::RecordData {
            allow: None,
            created_at: Datetime::now(),
            hidden_replies: None,
            post: root.uri.clone(),
        });
    let hidden = record.hidden_replies.get_or_insert_with(Vec::new);
    if hidden.iter().any(|uri| uri == &reply) {
        hidden.retain(|uri| uri != &reply);
    } else {
        hidden.push(reply);
    }
    record.put(agent, rkey(&root.uri)?).await?;
    Ok(())
}

pub async fn detach_quote(agent: &BskyAgent, post: String, quote: String) -> Result<()> {
    use atrium_api::app::bsky::feed::postgate;
    let key = rkey(&post)?;
    let existing = postgate::Record::get(agent, key.clone())
        .await
        .ok()
        .and_then(|output| postgate::Record::try_from_unknown(output.value.clone()).ok());
    let mut record = existing
        .map(|record| record.data)
        .unwrap_or(postgate::RecordData {
            created_at: Datetime::now(),
            detached_embedding_uris: None,
            embedding_rules: None,
            post,
        });
    let detached = record.detached_embedding_uris.get_or_insert_with(Vec::new);
    if !detached.iter().any(|uri| uri == &quote) {
        detached.push(quote);
    }
    record.put(agent, key).await?;
    Ok(())
}

pub async fn starter_packs(agent: &BskyAgent, did: Did) -> Result<Vec<FeatureRow>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_actor_starter_packs(
            get_actor_starter_packs::ParametersData {
                actor: did.clone().into(),
                cursor: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .starter_packs
        .iter()
        .map(|pack| {
            let record = starterpack::Record::try_from_unknown(pack.record.clone()).ok();
            FeatureRow {
                title: record
                    .as_ref()
                    .map(|record| record.name.clone())
                    .unwrap_or_else(|| "Starter Pack".into()),
                detail: format!(
                    "{} members · {} joins",
                    pack.list_item_count.unwrap_or(0),
                    pack.joined_all_time_count.unwrap_or(0)
                ),
                target: FeatureTarget::StarterPack {
                    uri: pack.uri.clone(),
                    cid: pack.cid.clone(),
                    owned: pack.creator.did == did,
                },
                unread: false,
            }
        })
        .collect())
}

pub async fn create_starter_pack(
    agent: &BskyAgent,
    name: String,
    description: Option<String>,
    list: String,
) -> Result<String> {
    if name.trim().is_empty() {
        bail!("starter pack name cannot be empty");
    }
    let output = starterpack::RecordData {
        created_at: Datetime::now(),
        description,
        description_facets: None,
        feeds: None,
        list,
        name,
    }
    .create(agent)
    .await?;
    Ok(output.uri.clone())
}

pub async fn edit_starter_pack(
    agent: &BskyAgent,
    uri: &str,
    name: String,
    description: Option<String>,
    list: String,
) -> Result<()> {
    starterpack::RecordData {
        created_at: Datetime::now(),
        description,
        description_facets: None,
        feeds: None,
        list,
        name,
    }
    .put(agent, rkey(uri)?)
    .await?;
    Ok(())
}

pub async fn starter_pack_detail(agent: &BskyAgent, uri: String) -> Result<Vec<FeatureRow>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_starter_pack(
            atrium_api::app::bsky::graph::get_starter_pack::ParametersData { starter_pack: uri }
                .into(),
        )
        .await?;
    let items = if let Some(list) = &output.starter_pack.list {
        agent
            .api
            .app
            .bsky
            .graph
            .get_list(
                get_list::ParametersData {
                    cursor: None,
                    limit: None,
                    list: list.uri.clone(),
                }
                .into(),
            )
            .await?
            .items
            .clone()
    } else {
        output
            .starter_pack
            .list_items_sample
            .clone()
            .unwrap_or_default()
    };
    Ok(items
        .iter()
        .map(|item| FeatureRow {
            title: format!("@{}", item.subject.handle.as_str()),
            detail: item.subject.display_name.clone().unwrap_or_default(),
            target: FeatureTarget::Actor(item.subject.did.clone().into()),
            unread: false,
        })
        .collect())
}

pub async fn discovery(agent: &BskyAgent, did: Did) -> Result<Vec<FeatureRow>> {
    let topics = agent
        .api
        .app
        .bsky
        .unspecced
        .get_trending_topics(
            get_trending_topics::ParametersData {
                limit: None,
                viewer: Some(did),
            }
            .into(),
        )
        .await?;
    let users = agent
        .api
        .app
        .bsky
        .unspecced
        .get_suggested_users(
            get_suggested_users::ParametersData {
                category: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    let mut rows = topics
        .topics
        .iter()
        .map(|topic| FeatureRow {
            title: format!("# {}", topic.topic),
            detail: topic.display_name.clone().unwrap_or_default(),
            target: FeatureTarget::Topic(topic.topic.clone()),
            unread: false,
        })
        .collect::<Vec<_>>();
    rows.extend(users.actors.iter().map(|actor| FeatureRow {
        title: format!(
            "Suggested · {} @{}",
            actor.display_name.as_deref().unwrap_or(""),
            actor.handle.as_str()
        ),
        detail: actor.description.clone().unwrap_or_default(),
        target: FeatureTarget::Actor(actor.did.clone().into()),
        unread: false,
    }));
    Ok(rows)
}

async fn chat_agent(agent: &BskyAgent) -> Result<BskyAgent> {
    let mut config = agent.to_config().await;
    config.proxy_header = Some(CHAT_PROXY.to_owned());
    Ok(BskyAgent::builder().config(config).build().await?)
}

pub async fn conversations(agent: &BskyAgent, own_did: &Did) -> Result<Vec<FeatureRow>> {
    let chat = chat_agent(agent).await?;
    let output = chat
        .api
        .chat
        .bsky
        .convo
        .list_convos(
            list_convos::ParametersData {
                cursor: None,
                limit: None,
                read_state: None,
                status: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .convos
        .iter()
        .map(|convo| convo_row(convo, own_did))
        .collect())
}

fn convo_row(convo: &convo_defs::ConvoView, own_did: &Did) -> FeatureRow {
    let members = convo
        .members
        .iter()
        .filter(|member| &member.did != own_did)
        .map(|member| format!("@{}", member.handle.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let preview = match &convo.last_message {
        Some(Union::Refs(convo_defs::ConvoViewLastMessageRefs::MessageView(message))) => {
            message.text.clone()
        }
        Some(_) => "[deleted message]".into(),
        None => "No messages".into(),
    };
    FeatureRow {
        title: format!(
            "{}{}",
            if convo.unread_count > 0 { "● " } else { "" },
            members
        ),
        detail: format!("{} unread · {}", convo.unread_count, preview),
        target: FeatureTarget::Conversation {
            id: convo.id.clone(),
            muted: convo.muted,
            members: convo
                .members
                .iter()
                .filter(|member| &member.did != own_did)
                .map(|member| member.did.clone())
                .collect(),
        },
        unread: convo.unread_count > 0,
    }
}

pub async fn conversation(agent: &BskyAgent, convo_id: String) -> Result<Vec<FeatureRow>> {
    let chat = chat_agent(agent).await?;
    let output = chat
        .api
        .chat
        .bsky
        .convo
        .get_messages(
            get_messages::ParametersData {
                convo_id: convo_id.clone(),
                cursor: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    let mut rows = output
        .messages
        .iter()
        .map(|message| match message {
            Union::Refs(get_messages::OutputMessagesItem::ChatBskyConvoDefsMessageView(
                message,
            )) => FeatureRow {
                title: format!(
                    "{} · {}",
                    message.sender.did.as_str(),
                    configured_datetime(message.sent_at.as_str())
                ),
                detail: message.text.clone(),
                target: FeatureTarget::Message {
                    convo_id: convo_id.clone(),
                    id: message.id.clone(),
                    sender: message.sender.did.clone(),
                },
                unread: false,
            },
            _ => FeatureRow {
                title: "[deleted message]".into(),
                detail: String::new(),
                target: FeatureTarget::Info,
                unread: false,
            },
        })
        .collect::<Vec<_>>();
    rows.reverse();
    chat.api
        .chat
        .bsky
        .convo
        .update_read(
            update_read::InputData {
                convo_id,
                message_id: None,
            }
            .into(),
        )
        .await?;
    Ok(rows)
}

pub async fn start_conversation(agent: &BskyAgent, member: Did) -> Result<String> {
    let chat = chat_agent(agent).await?;
    let availability = chat
        .api
        .chat
        .bsky
        .convo
        .get_convo_availability(
            atrium_api::chat::bsky::convo::get_convo_availability::ParametersData {
                members: vec![member.clone()],
            }
            .into(),
        )
        .await?;
    if !availability.can_chat {
        bail!("this account does not allow you to start a direct message");
    }
    if let Some(convo) = &availability.convo {
        return Ok(convo.id.clone());
    }
    let output = chat
        .api
        .chat
        .bsky
        .convo
        .get_convo_for_members(
            get_convo_for_members::ParametersData {
                members: vec![member],
            }
            .into(),
        )
        .await?;
    Ok(output.convo.id.clone())
}

pub async fn send_dm(agent: &BskyAgent, convo_id: String, text: String) -> Result<()> {
    if text.trim().is_empty() {
        bail!("message cannot be empty");
    }
    if text.chars().count() > 1000 {
        bail!("message is longer than 1000 characters");
    }
    let chat = chat_agent(agent).await?;
    chat.api
        .chat
        .bsky
        .convo
        .send_message(
            send_message::InputData {
                convo_id,
                message: convo_defs::MessageInputData {
                    embed: None,
                    facets: None,
                    text,
                }
                .into(),
            }
            .into(),
        )
        .await?;
    Ok(())
}

pub async fn toggle_conversation_mute(
    agent: &BskyAgent,
    convo_id: String,
    muted: bool,
) -> Result<()> {
    let chat = chat_agent(agent).await?;
    if muted {
        chat.api
            .chat
            .bsky
            .convo
            .unmute_convo(unmute_convo::InputData { convo_id }.into())
            .await?;
    } else {
        chat.api
            .chat
            .bsky
            .convo
            .mute_convo(mute_convo::InputData { convo_id }.into())
            .await?;
    }
    Ok(())
}

pub async fn set_incoming_dm(agent: &BskyAgent, value: String) -> Result<()> {
    if !matches!(value.as_str(), "all" | "none" | "following") {
        bail!("DM setting must be all, following, or none");
    }
    atrium_api::chat::bsky::actor::declaration::RecordData {
        allow_incoming: value,
    }
    .put(
        agent,
        RecordKey::new("self".to_owned()).map_err(eyre::Report::msg)?,
    )
    .await?;
    Ok(())
}

pub async fn moderation_preferences_rows(agent: &BskyAgent) -> Result<Vec<FeatureRow>> {
    let output = agent
        .api
        .app
        .bsky
        .actor
        .get_preferences(get_preferences::ParametersData {}.into())
        .await?;
    let mut rows = Vec::new();
    for preference in &output.preferences {
        let Union::Refs(preference) = preference else {
            continue;
        };
        match preference {
            PreferencesItem::MutedWordsPref(words) => {
                rows.extend(words.items.iter().map(|word| FeatureRow {
                    title: format!("Muted word · {}", word.value),
                    detail: format!("targets: {}", word.targets.join(", ")),
                    target: FeatureTarget::MutedWord(word.value.clone()),
                    unread: false,
                }))
            }
            PreferencesItem::ContentLabelPref(pref) => rows.push(FeatureRow {
                title: format!("Label · {}", pref.label),
                detail: format!("visibility: {}", pref.visibility),
                target: FeatureTarget::Labeler(
                    pref.labeler_did.clone().unwrap_or_else(default_labeler),
                ),
                unread: false,
            }),
            PreferencesItem::LabelersPref(pref) => {
                rows.extend(pref.labelers.iter().map(|labeler| FeatureRow {
                    title: format!("Labeler · {}", labeler.did.as_str()),
                    detail: "subscribed".into(),
                    target: FeatureTarget::Labeler(labeler.did.clone()),
                    unread: false,
                }))
            }
            _ => {}
        }
    }
    Ok(rows)
}

pub async fn labeler_detail(agent: &BskyAgent, did: Did) -> Result<Vec<FeatureRow>> {
    use atrium_api::app::bsky::labeler::get_services;
    let output = agent
        .api
        .app
        .bsky
        .labeler
        .get_services(
            get_services::ParametersData {
                detailed: Some(true),
                dids: vec![did.clone()],
            }
            .into(),
        )
        .await?;
    let mut rows = Vec::new();
    for view in &output.views {
        if let Union::Refs(get_services::OutputViewsItem::AppBskyLabelerDefsLabelerViewDetailed(
            view,
        )) = view
        {
            rows.extend(view.policies.label_values.iter().map(|label| FeatureRow {
                title: label.clone(),
                detail: "Press e to set ignore / warn / hide".into(),
                target: FeatureTarget::LabelSetting {
                    labeler: did.clone(),
                    label: label.clone(),
                },
                unread: false,
            }));
        }
    }
    Ok(rows)
}

pub async fn add_muted_word(agent: &BskyAgent, value: String) -> Result<()> {
    update_preferences(agent, |preferences| {
        let mut found = false;
        for preference in preferences.iter_mut() {
            if let Union::Refs(PreferencesItem::MutedWordsPref(words)) = preference {
                if !words.items.iter().any(|word| word.value == value) {
                    words.items.push(
                        atrium_api::app::bsky::actor::defs::MutedWordData {
                            actor_target: None,
                            expires_at: None,
                            id: None,
                            targets: vec!["content".into(), "tag".into()],
                            value: value.clone(),
                        }
                        .into(),
                    );
                }
                found = true;
            }
        }
        if !found {
            preferences.push(Union::Refs(PreferencesItem::MutedWordsPref(Box::new(
                atrium_api::app::bsky::actor::defs::MutedWordsPrefData {
                    items: vec![atrium_api::app::bsky::actor::defs::MutedWordData {
                        actor_target: None,
                        expires_at: None,
                        id: None,
                        targets: vec!["content".into(), "tag".into()],
                        value,
                    }
                    .into()],
                }
                .into(),
            ))));
        }
    })
    .await
}

pub async fn remove_muted_word(agent: &BskyAgent, value: &str) -> Result<()> {
    update_preferences(agent, |preferences| {
        for preference in preferences.iter_mut() {
            if let Union::Refs(PreferencesItem::MutedWordsPref(words)) = preference {
                words.items.retain(|word| word.value != value);
            }
        }
    })
    .await
}

pub async fn toggle_labeler(agent: &BskyAgent, did: Did) -> Result<()> {
    update_preferences(agent, |preferences| {
        let mut found_pref = false;
        for preference in preferences.iter_mut() {
            if let Union::Refs(PreferencesItem::LabelersPref(labelers)) = preference {
                found_pref = true;
                if labelers.labelers.iter().any(|item| item.did == did) {
                    labelers.labelers.retain(|item| item.did != did);
                } else {
                    labelers.labelers.push(
                        atrium_api::app::bsky::actor::defs::LabelerPrefItemData {
                            did: did.clone(),
                        }
                        .into(),
                    );
                }
            }
        }
        if !found_pref {
            preferences.push(Union::Refs(PreferencesItem::LabelersPref(Box::new(
                atrium_api::app::bsky::actor::defs::LabelersPrefData {
                    labelers: vec![
                        atrium_api::app::bsky::actor::defs::LabelerPrefItemData { did }.into(),
                    ],
                }
                .into(),
            ))));
        }
    })
    .await
}

pub async fn set_label_visibility(
    agent: &BskyAgent,
    labeler: Option<Did>,
    label: String,
    visibility: String,
) -> Result<()> {
    if !matches!(visibility.as_str(), "ignore" | "warn" | "hide") {
        bail!("visibility must be ignore, warn, or hide");
    }
    update_preferences(agent, |preferences| {
        preferences.retain(|preference| {
            !matches!(preference, Union::Refs(PreferencesItem::ContentLabelPref(pref)) if pref.label == label && pref.labeler_did == labeler)
        });
        preferences.push(Union::Refs(PreferencesItem::ContentLabelPref(Box::new(
            atrium_api::app::bsky::actor::defs::ContentLabelPrefData {
                label,
                labeler_did: labeler,
                visibility,
            }
            .into(),
        ))));
    })
    .await
}

pub async fn report(
    agent: &BskyAgent,
    subject: ReportSubject,
    reason_name: &str,
    details: Option<String>,
) -> Result<()> {
    use atrium_api::com::atproto::{admin::defs::RepoRefData, moderation, repo::strong_ref};
    let reason_type = match reason_name.trim().to_ascii_lowercase().as_str() {
        "spam" => moderation::defs::REASON_SPAM,
        "rude" | "harassment" => moderation::defs::REASON_RUDE,
        "sexual" => moderation::defs::REASON_SEXUAL,
        "violation" => moderation::defs::REASON_VIOLATION,
        "misleading" => moderation::defs::REASON_MISLEADING,
        "other" => moderation::defs::REASON_OTHER,
        _ => bail!("unknown report reason"),
    }
    .to_owned();
    let (subject, dm_context) = match subject {
        ReportSubject::Account(did) => (
            Union::Refs(
                moderation::create_report::InputSubjectRefs::ComAtprotoAdminDefsRepoRef(Box::new(
                    RepoRefData { did }.into(),
                )),
            ),
            None,
        ),
        ReportSubject::Record { uri, cid } => (
            Union::Refs(
                moderation::create_report::InputSubjectRefs::ComAtprotoRepoStrongRefMain(Box::new(
                    strong_ref::MainData { uri, cid }.into(),
                )),
            ),
            None,
        ),
        ReportSubject::Feed(uri) => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_feed_generator(
                    atrium_api::app::bsky::feed::get_feed_generator::ParametersData {
                        feed: uri.clone(),
                    }
                    .into(),
                )
                .await?;
            (
                Union::Refs(
                    moderation::create_report::InputSubjectRefs::ComAtprotoRepoStrongRefMain(
                        Box::new(
                            strong_ref::MainData {
                                uri,
                                cid: output.view.cid.clone(),
                            }
                            .into(),
                        ),
                    ),
                ),
                None,
            )
        }
        ReportSubject::Conversation {
            convo_id,
            message_id,
            sender,
        } => (
            Union::Refs(
                moderation::create_report::InputSubjectRefs::ComAtprotoAdminDefsRepoRef(Box::new(
                    RepoRefData { did: sender }.into(),
                )),
            ),
            Some(format!(
                "DM conversation {convo_id}, message {}",
                message_id.as_deref().unwrap_or("unspecified")
            )),
        ),
    };
    let reason = match (dm_context, details.filter(|value| !value.trim().is_empty())) {
        (Some(context), Some(details)) => Some(format!("{context}: {details}")),
        (Some(context), None) => Some(context),
        (None, details) => details,
    };
    agent
        .api
        .com
        .atproto
        .moderation
        .create_report(
            moderation::create_report::InputData {
                mod_tool: None,
                reason,
                reason_type,
                subject,
            }
            .into(),
        )
        .await?;
    Ok(())
}

async fn update_preferences(
    agent: &BskyAgent,
    mutate: impl FnOnce(&mut Vec<Union<PreferencesItem>>),
) -> Result<()> {
    let mut preferences = agent
        .api
        .app
        .bsky
        .actor
        .get_preferences(get_preferences::ParametersData {}.into())
        .await?
        .preferences
        .clone();
    mutate(&mut preferences);
    agent
        .api
        .app
        .bsky
        .actor
        .put_preferences(put_preferences::InputData { preferences }.into())
        .await?;
    Ok(())
}

fn default_labeler() -> Did {
    Did::new("did:plc:ar7c4by46qjdydhdevvrndac".to_owned()).expect("static DID is valid")
}

fn configured_datetime(value: &str) -> String {
    let format = crate::app::config::AppConfig::load()
        .map(|config| config.ui.date_format)
        .unwrap_or_else(|_| "%Y-%m-%d %H:%M".into());
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.format(&format).to_string())
        .unwrap_or_else(|_| value.to_owned())
}

fn rkey(uri: &str) -> Result<RecordKey> {
    let value = uri
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| eyre!("invalid AT URI"))?;
    RecordKey::new(value.to_owned()).map_err(eyre::Report::msg)
}
