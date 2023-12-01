use super::scalars::*;
use super::QueryUnwrap;
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug, Copy, Clone)]
pub struct TournamentEventsVars<'a> {
    pub after_date: Timestamp,
    pub before_date: Timestamp,

    pub game_id: VideogameId,
    pub country: Option<&'a str>,
    pub state: Option<&'a str>,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "TournamentEventsVars")]
pub struct TournamentEvents {
    #[arguments(query: {
        page: 1,
        perPage: 225,
        sortBy: "startAt asc",
        filter: {
            past: true,
            afterDate: $after_date,
            beforeDate: $before_date,
            videogameIds: [$game_id],
            countryCode: $country,
            addrState: $state
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
    id: Option<TournamentId>,
    start_at: Option<Timestamp>,
    #[arguments(limit: 99999, filter: { videogameId: [$game_id] })]
    #[cynic(flatten)]
    events: Vec<Event>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentEventsVars")]
struct Event {
    id: Option<EventId>,
    slug: Option<String>,
    start_at: Option<Timestamp>,
}

// Unwrap

#[derive(Debug, Clone)]
pub struct TournamentData {
    pub id: TournamentId,
    pub time: Timestamp,
    pub events: Vec<EventData>,
}

#[derive(Debug, Clone)]
pub struct EventData {
    pub id: EventId,
    pub slug: String,
    pub time: Timestamp,
}

impl<'a> QueryUnwrap<TournamentEventsVars<'a>> for TournamentEvents {
    type Unwrapped = Vec<TournamentData>;

    fn unwrap_response(response: GraphQlResponse<TournamentEvents>) -> Option<Vec<TournamentData>> {
        let response_tournaments = response.data?.tournaments?;

        Some(
            response_tournaments
                .nodes
                .into_iter()
                .filter_map(|tour| {
                    Some(TournamentData {
                        id: tour.id?,
                        time: tour.start_at?,
                        events: tour
                            .events
                            .into_iter()
                            .filter_map(|event| {
                                Some(EventData {
                                    id: event.id?,
                                    slug: event.slug?,
                                    time: event.start_at?,
                                })
                            })
                            .collect(),
                    })
                })
                .collect::<Vec<_>>(),
        )
    }
}
