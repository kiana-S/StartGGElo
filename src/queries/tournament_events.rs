use super::QueryUnwrap;
use super::{EventId, Timestamp, VideogameId};
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug)]
pub struct TournamentEventsVars {
    // HACK: This should really be an optional variable, but there seems to be a
    // server-side bug that completely breaks everything when this isn't passed.
    // We can use a dummy value of 1 when we don't want to filter by time.
    pub last_query: Timestamp,
    pub game_id: VideogameId,
    pub page: i32,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "TournamentEventsVars")]
pub struct TournamentEvents {
    #[arguments(query: {
        page: $page,
        perPage: 300,
        sortBy: "endAt asc",
        filter: {
            past: true,
            afterDate: $last_query,
            videogameIds: [$game_id],
        }})]
    tournaments: Option<TournamentConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentEventsVars")]
struct TournamentConnection {
    #[cynic(flatten)]
    nodes: Vec<Tournament>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentEventsVars")]
struct Tournament {
    name: Option<String>,
    #[arguments(limit: 99999, filter: { videogameId: [$game_id] })]
    #[cynic(flatten)]
    events: Vec<Event>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentEventsVars")]
struct Event {
    id: Option<EventId>,
}

// Unwrap

#[derive(Debug, Clone)]
pub struct TournamentData {
    pub name: String,
    pub events: Vec<EventId>,
}

impl QueryUnwrap<TournamentEventsVars> for TournamentEvents {
    type Unwrapped = Vec<TournamentData>;

    fn unwrap_response(response: GraphQlResponse<TournamentEvents>) -> Option<Vec<TournamentData>> {
        Some(
            response
                .data?
                .tournaments?
                .nodes
                .into_iter()
                .filter_map(|tour| {
                    Some(TournamentData {
                        name: tour.name?,
                        events: tour
                            .events
                            .into_iter()
                            .filter_map(|event| event.id)
                            .collect(),
                    })
                })
                .collect(),
        )
    }
}
