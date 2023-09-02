use super::{EntrantId, QueryUnwrap, Timestamp, VideogameId};
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug)]
pub struct TournamentSetsVars<'a> {
    // HACK: This should really be an optional variable, but there seems to be a
    // server-side bug that completely breaks everything when this isn't passed.
    pub last_query: Timestamp,

    pub country: Option<&'a str>,
    pub game_id: VideogameId,
    pub state: Option<&'a str>,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "TournamentSetsVars")]
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
#[cynic(variables = "TournamentSetsVars")]
struct TournamentConnection {
    #[cynic(flatten)]
    nodes: Vec<Tournament>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentSetsVars")]
struct Tournament {
    name: Option<String>,
    #[arguments(limit: 1000, filter: { videogameId: [$game_id] })]
    #[cynic(flatten)]
    events: Vec<Event>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Event {
    #[arguments(page: 1, perPage: 999)]
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
}

// Unwrap

pub struct TournamentResponse {
    pub name: String,
    pub sets: Vec<SetResponse>,
}

pub struct SetResponse {
    pub player1: EntrantId,
    pub player2: EntrantId,
    pub winner: bool,
}

impl<'a> QueryUnwrap<TournamentSetsVars<'a>> for TournamentSets {
    type VarsUnwrapped = TournamentSetsVars<'a>;
    type Unwrapped = Vec<TournamentResponse>;

    fn wrap_vars(vars: TournamentSetsVars) -> TournamentSetsVars {
        vars
    }

    // This might be the most spaghetti code I've ever written
    fn unwrap_response(
        response: GraphQlResponse<TournamentSets>,
    ) -> Option<Vec<TournamentResponse>> {
        Some(
            response
                .data?
                .tournaments?
                .nodes
                .into_iter()
                .filter_map(|tour| {
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
                                        let slots = set.slots;
                                        let player1 = (&slots[0]).entrant.as_ref()?.id?;
                                        let player2 = (&slots[0]).entrant.as_ref()?.id?;
                                        let winner = set.winner_id? as u64;
                                        Some(SetResponse {
                                            player1,
                                            player2,
                                            winner: winner == player2.0,
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .flatten()
                        .collect();
                    Some(TournamentResponse {
                        name: tour.name?,
                        sets,
                    })
                })
                .collect(),
        )
    }
}
