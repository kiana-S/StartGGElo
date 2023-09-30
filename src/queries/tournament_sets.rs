use super::{EntrantId, PlayerData, PlayerId, QueryUnwrap, Timestamp, VideogameId};
use cynic::GraphQlResponse;
use schema::schema;

pub type Teams<T> = Vec<Vec<T>>;

// Variables

#[derive(cynic::QueryVariables, Debug)]
pub struct TournamentSetsVars {
    // HACK: This should really be an optional variable, but there seems to be a
    // server-side bug that completely breaks everything when this isn't passed.
    // We can use a dummy value of 1 when we don't want to filter by time.
    pub last_query: Timestamp,
    pub game_id: VideogameId,

    pub tournament: i32,
    pub event_limit: i32,
    pub set_page: i32,
    pub set_pagesize: i32,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "TournamentSetsVars")]
pub struct TournamentSets {
    #[arguments(query: {
        page: $tournament,
        perPage: 1,
        sortBy: "endAt asc",
        filter: {
            past: true,
            afterDate: $last_query,
            videogameIds: [$game_id],
        }})]
    tournaments: Option<TournamentConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentSetsVars")]
struct TournamentConnection {
    #[cynic(flatten)]
    nodes: Vec<Tournament>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentSetsVars")]
struct Tournament {
    name: Option<String>,
    #[arguments(limit: $event_limit, filter: { videogameId: [$game_id] })]
    #[cynic(flatten)]
    events: Vec<Event>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentSetsVars")]
struct Event {
    #[arguments(page: $set_page, perPage: $set_pagesize)]
    sets: Option<SetConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
struct SetConnection {
    #[cynic(flatten)]
    nodes: Vec<Set>,
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

#[derive(Debug, Clone)]
pub struct TournamentData {
    pub name: String,
    pub sets: Vec<SetData>,
}

#[derive(Debug, Clone)]
pub struct SetData {
    pub teams: Teams<PlayerData>,
    pub winner: usize,
}

impl QueryUnwrap<TournamentSetsVars> for TournamentSets {
    type Unwrapped = TournamentData;

    // This might be the most spaghetti code I've ever written
    fn unwrap_response(response: GraphQlResponse<TournamentSets>) -> Option<TournamentData> {
        let tour = response.data?.tournaments?.nodes.into_iter().next()?;
        let sets = tour
            .events
            .into_iter()
            .filter_map(|event| {
                Some(
                    event
                        .sets?
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
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();
        Some(TournamentData {
            name: tour.name?,
            sets,
        })
    }
}
