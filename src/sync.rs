use std::thread::sleep;
use std::time::Duration;

use crate::datasets::*;
use crate::error;
use crate::queries::*;
use sqlite::*;

// Score calculation

/// Calculate the collective expected score for each team.
fn expected_scores(ratings: &Teams<&mut f64>) -> Vec<f64> {
    let qs: Vec<f64> = ratings
        .into_iter()
        .map(|es| 10_f64.powf(es.iter().map(|x| **x).sum::<f64>() / es.len() as f64 / 400.0))
        .collect();
    let sumq: f64 = qs.iter().sum();
    qs.into_iter().map(|q| q / sumq).collect()
}

/// Adjust the ratings of each player based on who won.
fn adjust_ratings(ratings: Teams<&mut f64>, winner: usize) {
    let exp_scores = expected_scores(&ratings);

    ratings
        .into_iter()
        .zip(exp_scores.into_iter())
        .enumerate()
        .for_each(|(i, (es, exp_sc))| {
            let len = es.len() as f64;
            let score = f64::from(winner == i);
            es.into_iter()
                .for_each(|e| *e += 40.0 * (score - exp_sc) / len);
        })
}

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
        println!("  (Page 1)");

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

fn get_tournament_events(dataset_config: &DatasetConfig, auth: &str) -> Option<Vec<EventId>> {
    println!("Accessing tournaments...");

    let tour_response = run_query::<TournamentEvents, _>(
        TournamentEventsVars {
            last_sync: dataset_config.last_sync,
            game_id: dataset_config.game_id,
            state: dataset_config.state.as_deref(),
            page: 1,
        },
        auth,
    )?;

    let pages = tour_response.pages;
    if pages == 0 {
        Some(vec![])
    } else if pages == 1 {
        Some(
            tour_response
                .tournaments
                .into_iter()
                .flat_map(|tour| tour.events)
                .collect::<Vec<_>>(),
        )
    } else {
        println!("  (Page 1)");

        let mut tournaments = tour_response
            .tournaments
            .into_iter()
            .flat_map(|tour| tour.events)
            .collect::<Vec<_>>();

        for page in 2..=pages {
            println!("  (Page {})", page);

            let next_response = run_query::<TournamentEvents, _>(
                TournamentEventsVars {
                    last_sync: dataset_config.last_sync,
                    game_id: dataset_config.game_id,
                    state: dataset_config.state.as_deref(),
                    page,
                },
                auth,
            )?;

            tournaments.extend(
                next_response
                    .tournaments
                    .into_iter()
                    .flat_map(|tour| tour.events),
            );
        }

        Some(tournaments)
    }
}

// Dataset syncing

fn update_from_set(connection: &Connection, dataset: &str, results: SetData) -> sqlite::Result<()> {
    let players_data = results.teams;
    add_players(connection, dataset, &players_data)?;

    let mut elos = get_ratings(connection, dataset, &players_data)?;
    adjust_ratings(
        elos.iter_mut()
            .map(|v| v.iter_mut().map(|x| &mut x.1).collect())
            .collect(),
        results.winner,
    );
    update_ratings(connection, dataset, elos)
}

pub fn sync_dataset(
    connection: &Connection,
    dataset: &str,
    dataset_config: DatasetConfig,
    auth: &str,
) -> sqlite::Result<()> {
    let events = get_tournament_events(&dataset_config, auth)
        .unwrap_or_else(|| error("Could not access start.gg", 1));

    connection.execute("BEGIN;")?;

    let num_events = events.len();
    for (i, event) in events.into_iter().enumerate() {
        println!(
            "Accessing sets from event ID {}... ({}/{})",
            event.0, i, num_events
        );

        let sets = get_event_sets(event, auth).unwrap_or_else(|| {
            connection.execute("ROLLBACK;").unwrap();
            error("Could not access start.gg", 1)
        });

        println!("  Updating ratings from event...");

        sets.into_iter()
            .try_for_each(|set| update_from_set(connection, dataset, set))?;
    }
    connection.execute("COMMIT;")
}
