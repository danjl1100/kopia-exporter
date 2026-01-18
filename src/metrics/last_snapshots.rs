use crate::{KopiaSnapshots, Snapshot, SourceMap, SourceStr, metrics::DisplayMetric};
use std::fmt::{self, Display};

#[derive(Clone, Copy)]
struct LastSnapshots<'a> {
    map: &'a SourceMap<Vec<Snapshot>>,
}
impl<'a> LastSnapshots<'a> {
    fn new(map: &'a SourceMap<Vec<Snapshot>>) -> Option<Self> {
        map.iter()
            .any(|(_source, snapshots)| !snapshots.is_empty())
            .then_some(Self { map })
    }
    fn iter(self) -> impl Iterator<Item = (&'a SourceStr, &'a Snapshot)> {
        let Self { map } = self;
        map.iter()
            .filter_map(|(source, snapshots)| snapshots.last().map(|last| (source, last)))
    }
}

pub struct MetricLastSnapshots<'a, F> {
    last_snapshots: LastSnapshots<'a>,
    stat_fn: F,
}
impl<'a, F, T> MetricLastSnapshots<'a, F>
where
    F: Fn(&Snapshot) -> T,
    T: Display,
{
    pub fn new(ks: &'a KopiaSnapshots, stat_fn: F) -> Option<Self> {
        let last_snapshots = LastSnapshots::new(&ks.snapshots_map)?;
        Some(Self {
            last_snapshots,
            stat_fn,
        })
    }
}
impl<F, T> DisplayMetric for MetricLastSnapshots<'_, F>
where
    F: Fn(&Snapshot) -> T,
    T: Display,
{
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            last_snapshots,
            stat_fn,
        } = self;
        for (source, last) in last_snapshots.iter() {
            let stat = stat_fn(last);
            writeln!(f, "{name}{{source={source:?}}} {stat}")?;
        }
        Ok(())
    }
}
