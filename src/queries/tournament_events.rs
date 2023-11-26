use super::scalars::*;
use super::QueryUnwrap;
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug, Copy, Clone)]
pub struct TournamentEventsVars<'a> {
    // HACK: This should really be an optional variable, but there seems to be a
    // server-side bug that completely breaks everything when this isn't passed.
    // We can use a dummy value of 1 when we don't want to filter by time.
    pub last_sync: Timestamp,

    pub game_id: VideogameId,
    pub country: Option<&'a str>,
    pub state: Option<&'a str>,
    pub page: i32,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "TournamentEventsVars")]
pub struct TournamentEvents {
    #[arguments(query: {
        page: $page,
        perPage: 225,
        sortBy: "endAt asc",
        filter: {
            past: true,
            afterDate: $last_sync,
            videogameIds: [$game_id],
            countryCode: $country,
            addrState: $state
        }})]
    tournaments: Option<TournamentConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentEventsVars")]
struct TournamentConnection {
    page_info: Option<PageInfo>,
    #[cynic(flatten)]
    nodes: Vec<Tournament>,
}

#[derive(cynic::QueryFragment, Debug)]
struct PageInfo {
    total_pages: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(variables = "TournamentEventsVars")]
struct Tournament {
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
pub struct TournamentEventResponse {
    pub pages: i32,
    pub tournaments: Vec<TournamentData>,
}

#[derive(Debug, Clone)]
pub struct TournamentData {
    pub events: Vec<EventData>,
}

#[derive(Debug, Clone)]
pub struct EventData {
    pub id: EventId,
    pub slug: String,
    pub time: Timestamp,
}

impl<'a> QueryUnwrap<TournamentEventsVars<'a>> for TournamentEvents {
    type Unwrapped = TournamentEventResponse;

    fn unwrap_response(
        response: GraphQlResponse<TournamentEvents>,
    ) -> Option<TournamentEventResponse> {
        let response_tournaments = response.data?.tournaments?;

        let tournaments = response_tournaments
            .nodes
            .into_iter()
            .filter_map(|tour| {
                Some(TournamentData {
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
            .collect::<Vec<_>>();

        Some(TournamentEventResponse {
            pages: response_tournaments.page_info?.total_pages?,
            tournaments,
        })
    }
}
