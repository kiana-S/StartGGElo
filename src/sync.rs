use std::f64::consts::PI;
use std::thread::sleep;
use std::time::Duration;

use crate::datasets::*;
use crate::error;
use crate::queries::*;
use sqlite::*;

// Glicko-2 system calculation

fn time_adjust(periods: f64, old_dev_sq: f64, volatility: f64) -> f64 {
    (old_dev_sq + periods * volatility * volatility).sqrt()
}

fn illinois_optimize(fun: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let mut f_a = fun(a);
    let mut f_b = fun(b);

    while (b - a).abs() > 1e-6 {
        let c = a + (a - b) * f_a / (f_b - f_a);
        let f_c = fun(c);
        if f_c * f_b > 0.0 {
            f_a = f_a / 2.0;
        } else {
            a = b;
            f_a = f_b;
        }
        b = c;
        f_b = f_c;
    }
    a
}

fn glicko_adjust(
    advantage: f64,
    deviation: f64,
    volatility: f64,
    other_deviation: f64,
    won: bool,
    time: u64,
) -> (f64, f64, f64) {
    // TODO: Turn this into dataset metadata
    let tau = 0.2;
    let period = (3600 * 24 * 30) as f64;

    let g_val = 1.0 / (1.0 + 3.0 * other_deviation * other_deviation / PI / PI).sqrt();
    let exp_val = 1.0 / (1.0 + f64::exp(-g_val * advantage));

    let variance = 1.0 / (g_val * g_val * exp_val * (1.0 - exp_val));

    let score = if won { 1.0 } else { 0.0 };
    let delta = variance * g_val * (score - exp_val);

    let delta_sq = delta * delta;
    let dev_sq = deviation * deviation;
    let a = (volatility * volatility).ln();
    let vol_fn = |x| {
        let ex = f64::exp(x);
        let subf = dev_sq + variance + ex;
        ((ex * (delta_sq - dev_sq - variance - ex)) / 2.0 / subf / subf) - (x - a) / tau / tau
    };

    let initial_b = if delta_sq > dev_sq + variance {
        (delta_sq - dev_sq - variance).ln()
    } else {
        (1..)
            .map(|k| vol_fn(a - k as f64 * tau))
            .find(|x| x >= &0.0)
            .unwrap()
    };
    let vol_new = f64::exp(illinois_optimize(vol_fn, a, initial_b) / 2.0);

    let dev_time = time_adjust(time as f64 / period as f64, dev_sq, vol_new);
    let dev_new = 1.0 / (1.0 / dev_time / dev_time + 1.0 / variance).sqrt();
    let adjust = dev_new * dev_new * g_val * (score - exp_val);

    (adjust, dev_new, vol_new)
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

fn get_tournament_events(metadata: &DatasetMetadata, auth: &str) -> Option<Vec<EventId>> {
    println!("Accessing tournaments...");

    let tour_response = run_query::<TournamentEvents, _>(
        TournamentEventsVars {
            last_sync: metadata.last_sync,
            game_id: metadata.game_id,
            state: metadata.state.as_deref(),
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
                    last_sync: metadata.last_sync,
                    game_id: metadata.game_id,
                    state: metadata.state.as_deref(),
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
    add_players(connection, dataset, &players_data, results.time)?;

    // Non-singles matches are currently not supported
    if players_data.len() != 2 || players_data[0].len() != 1 || players_data[1].len() != 1 {
        return Ok(());
    }

    let mut it = players_data.into_iter();
    let player1 = it.next().unwrap()[0].id;
    let player2 = it.next().unwrap()[0].id;
    drop(it);

    let (deviation1, volatility1, last_played1) = get_player_data(connection, dataset, player1)?;
    let (deviation2, volatility2, last_played2) = get_player_data(connection, dataset, player1)?;
    let advantage = match get_advantage(connection, dataset, player1, player2) {
        Err(e) => Err(e)?,
        Ok(None) => initialize_edge(connection, dataset, player1, player2)?,
        Ok(Some(adv)) => adv,
    };
    let (adjust1, dev_new1, vol_new1) = glicko_adjust(
        -advantage,
        deviation1,
        volatility1,
        deviation2,
        results.winner == 0,
        results.time.0 - last_played1.0,
    );
    let (adjust2, dev_new2, vol_new2) = glicko_adjust(
        advantage,
        deviation2,
        volatility2,
        deviation1,
        results.winner == 1,
        results.time.0 - last_played2.0,
    );

    set_player_data(
        connection,
        dataset,
        player1,
        results.time,
        dev_new1,
        vol_new1,
    )?;
    set_player_data(
        connection,
        dataset,
        player2,
        results.time,
        dev_new2,
        vol_new2,
    )?;

    adjust_advantages(connection, dataset, player1, 0.5 * adjust1)?;
    adjust_advantages(connection, dataset, player2, 0.5 * adjust2)?;
    adjust_advantage(
        connection,
        dataset,
        player1,
        player2,
        (1.0 - 0.5) * (adjust2 - adjust1),
    )
}

pub fn sync_dataset(
    connection: &Connection,
    dataset: &str,
    metadata: DatasetMetadata,
    auth: &str,
) -> sqlite::Result<()> {
    let events = get_tournament_events(&metadata, auth)
        .unwrap_or_else(|| error("Could not access start.gg", 1));

    connection.execute("BEGIN;")?;

    let num_events = events.len();
    for (i, event) in events.into_iter().enumerate() {
        println!(
            "Accessing sets from event ID {}... ({}/{})",
            event.0, i, num_events
        );

        let sets =
            get_event_sets(event, auth).unwrap_or_else(|| error("Could not access start.gg", 1));

        println!("  Updating ratings from event...");

        sets.into_iter()
            .try_for_each(|set| update_from_set(connection, dataset, set))?;
    }
    connection.execute("COMMIT;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::tests::*;

    #[test]
    fn glicko_single() -> sqlite::Result<()> {
        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;
        let players = players(2).into_iter().map(|x| vec![x]).collect();
        add_players(&connection, "test", &players, Timestamp(0))?;

        update_from_set(
            &connection,
            "test",
            SetData {
                time: Timestamp(0),
                teams: players,
                winner: 0,
            },
        )?;

        println!(
            "{:?}",
            get_advantage(&connection, "test", PlayerId(1), PlayerId(2))?.unwrap()
        );
        println!("{:?}", get_player_data(&connection, "test", PlayerId(1)));
        println!("{:?}", get_player_data(&connection, "test", PlayerId(2)));

        Ok(())
    }
}
