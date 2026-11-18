use crate::{briefs::BRIEFS, read, Job, Result, ARMS};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
struct Rating {
    id: String,
    // Tonal emphasis, structured rhythm, tactile texture, central focal
    // emphasis, and atmosphere. All dimensions independently rated 0..4.
    features: [u8; 5],
    fit: u8,
}

pub fn report(run: &Path, path: &Path) -> Result<()> {
    let jobs: Vec<Job> = read(&run.join("plan.json"))?;
    let ratings: Vec<Rating> = read(path)?;
    if ratings
        .iter()
        .any(|r| r.fit > 4 || r.features.iter().any(|v| *v > 4))
    {
        return Err("ratings must be integers from 0 to 4".into());
    }
    let ratings_count = ratings.len();
    let ratings: BTreeMap<_, _> = ratings.into_iter().map(|r| (r.id.clone(), r)).collect();
    if ratings.len() != ratings_count
        || jobs.len() != ratings.len()
        || jobs.iter().any(|job| !ratings.contains_key(&job.id))
    {
        return Err("ratings must cover every plan ID exactly once".into());
    }
    for arm in ARMS {
        let rows: Vec<_> = jobs.iter().filter(|job| job.arm == arm).collect();
        let mut correct = 0.;
        let mut task_fit = 0.;
        for test in &rows {
            let medium = BRIEFS[test.brief].0;
            let features = ratings[&test.id].features;
            task_fit += f64::from(ratings[&test.id].fit);
            let distance = |period| {
                let mut mean = [0.; 5];
                let mut n = 0.;
                for train in &rows {
                    if train.period != period || BRIEFS[train.brief].0 == medium {
                        continue;
                    }
                    n += 1.;
                    for (total, value) in mean.iter_mut().zip(ratings[&train.id].features) {
                        *total += f64::from(value);
                    }
                }
                mean.iter()
                    .zip(features)
                    .map(|(m, v)| (m / n - f64::from(v)).powi(2))
                    .sum::<f64>()
            };
            let actual = distance(test.period);
            let other = distance(1 - test.period);
            correct += if actual < other {
                1.
            } else if actual == other {
                0.5
            } else {
                0.
            };
        }
        println!(
            "{arm}: cross-medium period identification {:.1}% | task fit {:.2}/4 ({} proposals)",
            100. * correct / rows.len() as f64,
            task_fit / rows.len() as f64,
            rows.len()
        );
    }
    Ok(())
}
