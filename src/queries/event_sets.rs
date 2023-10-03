use super::{EntrantId, EventId, PlayerData, PlayerId, QueryUnwrap};
use cynic::GraphQlResponse;
use schema::schema;

pub type Teams<T> = Vec<Vec<T>>;

// Variables

#[derive(cynic::QueryVariables, Debug, Copy, Clone)]
pub struct EventSetsVars {
    pub event: EventId,
    pub page: i32,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "EventSetsVars")]
pub struct EventSets {
    #[arguments(id: $event)]
    event: Option<Event>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "EventSetsVars")]
struct Event {
    #[arguments(page: $page, perPage: 50)]
    sets: Option<SetConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
struct SetConnection {
    page_info: Option<PageInfo>,
    #[cynic(flatten)]
    nodes: Vec<Set>,
}

#[derive(cynic::QueryFragment, Debug)]
struct PageInfo {
    total_pages: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Set {
    #[arguments(includeByes: true)]
    #[cynic(flatten)]
    slots: Vec<SetSlot>,
    winner_id: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug)]
struct SetSlot {
    entrant: Option<Entrant>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Entrant {
    id: Option<EntrantId>,
    #[cynic(flatten)]
    participants: Vec<Participant>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Participant {
    player: Option<Player>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Player {
    id: Option<PlayerId>,
    gamer_tag: Option<String>,
    prefix: Option<String>,
}

// Unwrap

pub struct EventSetsResponse {
    pub pages: u64,
    pub sets: Vec<SetData>,
}

pub struct SetData {
    pub teams: Teams<PlayerData>,
    pub winner: usize,
}

impl QueryUnwrap<EventSetsVars> for EventSets {
    type Unwrapped = EventSetsResponse;

    // This might be the most spaghetti code I've ever written
    fn unwrap_response(response: GraphQlResponse<EventSets>) -> Option<EventSetsResponse> {
        let response_sets = response.data?.event?.sets?;

        let sets = response_sets
            .nodes
            .into_iter()
            .filter_map(|set| {
                let winner_id = set.winner_id?;
                let winner = set.slots.iter().position(|slot| {
                    slot.entrant
                        .as_ref()
                        .and_then(|x| x.id)
                        .map(|id| id.0 == winner_id as u64)
                        .unwrap_or(false)
                })?;
                let teams = set
                    .slots
                    .into_iter()
                    .map(|slot| {
                        slot.entrant?
                            .participants
                            .into_iter()
                            .map(|p| {
                                let p_ = p.player?;
                                Some(PlayerData {
                                    id: p_.id?,
                                    name: p_.gamer_tag,
                                    prefix: p_.prefix,
                                })
                            })
                            .try_collect()
                    })
                    .try_collect()?;
                Some(SetData { teams, winner })
            })
            .collect::<Vec<_>>();

        Some(EventSetsResponse {
            pages: response_sets.page_info?.total_pages? as u64,
            sets,
        })
    }
}
