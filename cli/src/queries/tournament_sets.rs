use super::{EntrantId, QueryUnwrap, Timestamp, VideogameId, ID};
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug)]
pub struct TournamentSetsVarsRaw<'a> {
    country: Option<&'a str>,
    game_id: ID,
    last_query: Option<Timestamp>,
    state: Option<&'a str>,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "TournamentSetsVarsRaw")]
pub struct TournamentSets {
    #[arguments(query: {
        page: 1,
        perPage: 1,
        sortBy: "startAt desc",
        filter: {
            past: true,
            afterDate: $last_query,
            addrState: $state,
            countryCode: $country,
            videogameIds: [$game_id]
        }})]
    tournaments: Option<TournamentConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentSetsVarsRaw")]
struct TournamentConnection {
    nodes: Option<Vec<Option<Tournament>>>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentSetsVarsRaw")]
struct Tournament {
    name: Option<String>,
    #[arguments(limit: 1000, filter: { videogameId: [$game_id] })]
    events: Option<Vec<Option<Event>>>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Event {
    #[arguments(page: 1, perPage: 999)]
    sets: Option<SetConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
struct SetConnection {
    nodes: Option<Vec<Option<Set>>>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Set {
    #[arguments(includeByes: true)]
    slots: Option<Vec<Option<SetSlot>>>,
    winner_id: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug)]
struct SetSlot {
    entrant: Option<Entrant>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Entrant {
    id: Option<ID>,
}

// Unwrap

pub struct TournamentSetsVars<'a> {
    pub country: Option<&'a str>,
    pub game_id: VideogameId,
    pub last_query: Option<Timestamp>,
    pub state: Option<&'a str>,
}

pub struct TournamentResponse {
    pub name: String,
    pub sets: Vec<SetResponse>,
}

pub struct SetResponse {
    pub player1: EntrantId,
    pub player2: EntrantId,
    pub winner: bool,
}

impl<'a> QueryUnwrap<TournamentSetsVarsRaw<'a>> for TournamentSets {
    type VarsUnwrapped = TournamentSetsVars<'a>;
    type Unwrapped = Vec<TournamentResponse>;

    fn wrap_vars(
        TournamentSetsVars {
            country,
            game_id: VideogameId(game_id),
            last_query,
            state,
        }: TournamentSetsVars,
    ) -> TournamentSetsVarsRaw {
        TournamentSetsVarsRaw {
            country,
            game_id: ID(game_id),
            last_query,
            state,
        }
    }

    // This might be the most spaghetti code I've ever written
    fn unwrap_response(
        response: GraphQlResponse<TournamentSets>,
    ) -> Option<Vec<TournamentResponse>> {
        Some(
            response
                .data?
                .tournaments?
                .nodes?
                .into_iter()
                .filter_map(|tour| {
                    let tour_ = tour?;
                    let sets = tour_
                        .events?
                        .into_iter()
                        .filter_map(|event| {
                            let event_ = event?;
                            Some(
                                event_
                                    .sets?
                                    .nodes?
                                    .into_iter()
                                    .filter_map(|set| {
                                        let set_ = set?;
                                        let slots = set_.slots?;
                                        let player1 = (&slots[0]).as_ref()?.entrant.as_ref()?.id?.0;
                                        let player2 = (&slots[0]).as_ref()?.entrant.as_ref()?.id?.0;
                                        let winner = set_.winner_id? as u64;
                                        Some(SetResponse {
                                            player1: EntrantId(player1),
                                            player2: EntrantId(player2),
                                            winner: winner == player2,
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .flatten()
                        .collect();
                    Some(TournamentResponse {
                        name: tour_.name?,
                        sets,
                    })
                })
                .collect(),
        )
    }
}
