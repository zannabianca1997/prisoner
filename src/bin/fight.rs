#![feature(never_type)]

use std::{
    cmp::Reverse,
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::Context;
use clap::Parser;
use lazy_regex::regex_captures;
use prisoner::{EloPool, EloPoolConfig, Weights};
use rand::{rngs::SmallRng, SeedableRng};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "2,3-0,1")]
    weights: ArgWeights,
    #[clap(short = 'p', long, default_value = "700")]
    starting_pts: usize,
    #[clap(short, long, default_value = "100")]
    scale: f64,
    #[clap(short, long, default_value = "32")]
    k_factor: f64,
    #[clap(short = 't', long, default_value = "100")]
    min_turns: usize,
    #[clap(short = 'T', long, default_value = "200")]
    max_turns: usize,

    /// Refresh time in seconds
    #[clap(short, long, default_value = "2")]
    refresh: u64,
    #[clap(long)]
    seed: Option<u64>,
}

#[derive(Debug, Clone)]
struct ArgWeights(Weights);
impl FromStr for ArgWeights {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, defect_defect, defect_collab_0, defect_collab_1, collab_collab) =
            regex_captures!(r"^(\d+),(\d+)\-(\d+),(\d+)", s)
                .context("The weights must be in the format `dd,dcw-dcl,cc`")?;
        Ok(ArgWeights(Weights {
            defect_defect: defect_defect.parse().context("Integer overflow in dd")?,
            defect_collab: (
                defect_collab_0.parse().context("Integer overflow in dcw")?,
                defect_collab_1.parse().context("Integer overflow in dcl")?,
            ),
            collab_collab: collab_collab.parse().context("Integer overflow in cc")?,
        }))
    }
}
impl From<&Args> for EloPoolConfig {
    fn from(
        Args {
            weights,
            starting_pts,
            scale,
            k_factor,
            min_turns,
            max_turns,
            ..
        }: &Args,
    ) -> Self {
        Self {
            weights: weights.0,
            starting_pts: *starting_pts,
            scale: *scale,
            k_factor: *k_factor,
            min_turns: *min_turns,
            max_turns: *max_turns,
        }
    }
}

fn main() -> anyhow::Result<!> {
    let args = Args::parse();

    let mut pool = EloPool::from(EloPoolConfig::from(&args));
    let Args { refresh, seed, .. } = args;
    let refresh = Duration::from_secs(refresh);

    let mut rng = if let Some(seed) = seed {
        SmallRng::seed_from_u64(seed)
    } else {
        SmallRng::from_entropy()
    };

    loop {
        print_pool(&pool)?;
        let start = Instant::now();
        while Instant::now() < start + refresh {
            pool.play(&mut rng);
        }
    }
}

fn print_pool(pool: &EloPool<rand::distributions::Uniform<usize>>) -> anyhow::Result<()> {
    let mut ratings = pool.ratings().to_owned();
    ratings.sort_by_key(|(_, r)| Reverse(*r));
    clearscreen::clear()?;
    for (player, rating) in ratings {
        println!("{}\t{}\t({})", player.name(), rating, player.description())
    }
    Ok(())
}
