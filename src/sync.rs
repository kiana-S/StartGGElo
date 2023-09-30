use crate::datasets::*;
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
    let sets = run_query::<EventSets, _>(EventSetsVars {
        event,
        sets_page: 1,
    });
}

/*
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

pub fn update_from_tournament(
    connection: &Connection,
    dataset: &str,
    results: TournamentData,
) -> sqlite::Result<()> {
    results
        .sets
        .into_iter()
        .try_for_each(|set| update_from_set(connection, dataset, set))
}
*/
