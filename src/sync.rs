use std::thread::sleep;
use std::time::Duration;

use crate::database::*;
use crate::error;
use crate::queries::*;
use itertools::Itertools;
use sqlite::*;

// Extract set data

fn get_event_sets(event: EventId, auth: &str) -> Option<Vec<SetData>> {
    sleep(Duration::from_millis(700));

    let sets_response = run_query::<EventSets, _>(EventSetsVars { event, page: 1 }, auth)?;

    let pages = sets_response.pages;
    if pages == 0 {
        Some(vec![])
    } else if pages == 1 {
        Some(sets_response.sets)
    } else {
        let mut sets = sets_response.sets;

        for page in 2..=pages {
            println!("  (Page {})", page);

            let next_response = run_query::<EventSets, _>(
                EventSetsVars {
                    event,
                    page: page as i32,
                },
                auth,
            )?;

            sleep(Duration::from_millis(700));

            sets.extend(next_response.sets);
        }

        Some(sets)
    }
}

fn get_tournament_events(
    metadata: &DatasetMetadata,
    before: Timestamp,
    auth: &str,
) -> Option<Vec<EventData>> {
    println!("Accessing tournaments...");

    let mut after = metadata.last_sync;

    let tour_response = run_query::<TournamentEvents, _>(
        TournamentEventsVars {
            after_date: after,
            before_date: before,
            game_id: metadata.game_id,
            country: metadata.country.as_deref(),
            state: metadata.state.as_deref(),
        },
        auth,
    )?;

    let mut cont = !tour_response.is_empty();
    after = if tour_response.iter().any(|tour| tour.time != after) {
        tour_response.last().unwrap().time
    } else {
        Timestamp(after.0 + 1)
    };

    let mut tournaments = tour_response;

    let mut page: u64 = 1;
    while cont {
        page += 1;
        println!("  (Page {})", page);

        let next_response = run_query::<TournamentEvents, _>(
            TournamentEventsVars {
                after_date: after,
                before_date: before,
                game_id: metadata.game_id,
                country: metadata.country.as_deref(),
                state: metadata.state.as_deref(),
            },
            auth,
        )?;

        cont = !next_response.is_empty();
        after = if next_response.iter().any(|tour| tour.time != after) {
            next_response.last().unwrap().time
        } else {
            Timestamp(after.0 + 1)
        };

        tournaments.extend(next_response);
    }

    println!("Deduplicating...");

    Some(
        tournaments
            .into_iter()
            .group_by(|tour| tour.time)
            .into_iter()
            .flat_map(|(_, group)| group.into_iter().unique_by(|tour| tour.id))
            .flat_map(|tour| tour.events)
            .collect::<Vec<_>>(),
    )
}

// Dataset syncing

fn update_from_set(
    connection: &Connection,
    dataset: &str,
    metadata: &DatasetMetadata,
    event_time: Timestamp,
    results: SetData,
) -> sqlite::Result<()> {
    let teams = results.teams;

    // Non-singles matches are currently not supported
    if teams.len() != 2 || teams[0].len() != 1 || teams[1].len() != 1 {
        return Ok(());
    }

    let players = teams.into_iter().flatten().collect::<Vec<_>>();
    add_players(connection, dataset, &players)?;

    let player1 = players[0].id;
    let player2 = players[1].id;

    // Time-adjust all variances associated with each player
    let time = results.time.unwrap_or(event_time);
    adjust_for_time(connection, dataset, player1, metadata.var_const, time)?;
    adjust_for_time(connection, dataset, player2, metadata.var_const, time)?;

    let (advantage, variance) = match get_network_data(connection, dataset, player1, player2) {
        Err(e) => Err(e)?,
        Ok(None) => initialize_edge(
            connection,
            dataset,
            player1,
            player2,
            metadata.decay_const,
            time,
        )?,
        Ok(Some(adv)) => adv,
    };

    // println!("{}, {} - {}, {}", player1.0, player2.0, advantage, variance);

    glicko_adjust(
        connection,
        dataset,
        &results.id,
        player1,
        player2,
        advantage,
        variance,
        results.winner,
        metadata.decay_const,
    )?;

    set_player_set_counts(
        connection,
        dataset,
        player1,
        results.winner == 0,
        &results.id,
    )?;
    set_player_set_counts(
        connection,
        dataset,
        player2,
        results.winner == 1,
        &results.id,
    )?;

    Ok(())
}

pub fn sync_dataset(
    connection: &Connection,
    dataset: &str,
    metadata: DatasetMetadata,
    before: Timestamp,
    auth: &str,
) -> sqlite::Result<()> {
    let events = get_tournament_events(&metadata, before, auth)
        .unwrap_or_else(|| error("Could not access start.gg", 1));

    connection.execute("BEGIN;")?;

    let num_events = events.len();
    for (i, event) in events.into_iter().enumerate() {
        println!(
            "Accessing sets from event ID {}... ({}/{})",
            event.id.0,
            i + 1,
            num_events
        );

        add_event(connection, event.id, &event.slug)?;

        let mut sets =
            get_event_sets(event.id, auth).unwrap_or_else(|| error("Could not access start.gg", 1));

        if sets.is_empty() {
            println!("  No valid sets");
        } else {
            println!("  Updating ratings from event...");

            sets.sort_by_key(|set| set.time);
            sets.into_iter().try_for_each(|set| {
                add_set(connection, &set.id, event.id)?;
                update_from_set(connection, dataset, &metadata, event.time, set)
            })?;
        }
    }
    connection.execute("COMMIT;")
}
